use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        bundle::Bundle,
        component::Component,
        query::With,
        system::{Commands, Query, Resource},
        world::{EntityRef, EntityWorldMut, World},
    },
    hierarchy::DespawnRecursiveExt,
};
use crossbeam::channel::{bounded, Receiver, Sender};
use opencv::core::Mat;
use tracing::error;

use crate::video_stream::{VideoProcessor, VideoProcessorFactory};

pub trait AppPipelineExt {
    fn register_video_pipeline<P>(&mut self) -> &mut Self
    where
        P: Pipeline + Default;
}

impl AppPipelineExt for App {
    fn register_video_pipeline<P>(&mut self) -> &mut Self
    where
        P: Pipeline + Default,
    {
        self.add_systems(Update, pipeline_sync::<P>);

        self.init_resource::<VideoPipelines>();
        self.world
            .resource_mut::<VideoPipelines>()
            .0
            .push(VideoPipeline {
                name: P::NAME,
                factory: factory::<P>(),
            });

        self
    }
}

#[derive(Resource, Default)]
pub struct VideoPipelines(pub Vec<VideoPipeline>);
pub struct VideoPipeline {
    pub name: &'static str,
    pub factory: VideoProcessorFactory,
}

pub type EntityWorldCallback = Box<dyn FnOnce(EntityWorldMut) + Send + Sync + 'static>;

pub trait Pipeline: Send + Sync + 'static {
    const NAME: &'static str;

    type Input: Default + Send + Sync + 'static;

    fn collect_inputs(world: &World, entity: &EntityRef) -> Self::Input;
    // TODO: better api for writing back to the ECS
    fn process<'a>(
        &'a mut self,
        cmds: &Sender<EntityWorldCallback>,
        data: &Self::Input,
        img: &Mat,
    ) -> &'a Mat;
    /// Entity is implicitly despawned after this function returns
    fn cleanup(entity_world: &mut EntityWorldMut);
}

pub fn factory<P: Pipeline + Default>() -> VideoProcessorFactory {
    VideoProcessorFactory(|world| {
        let (handler, bundle) = PipelineHandler::new(P::default());
        world.spawn(bundle);

        Box::new(handler)
    })
}

type ArcMutArc<T> = Arc<Mutex<Arc<T>>>;

pub struct PipelineHandler<P: Pipeline> {
    pipeline: P,

    input: ArcMutArc<P::Input>,
    output_tx: Sender<EntityWorldCallback>,
}

impl<P: Pipeline> PipelineHandler<P> {
    pub fn new(pipeline: P) -> (Self, PipelineBundle<P>) {
        let input: ArcMutArc<P::Input> = Default::default();
        let (output_tx, output_rx) = bounded(20);

        (
            Self {
                pipeline,
                input: input.clone(),
                output_tx,
            },
            PipelineBundle {
                channels: PipelineChannels { input, output_rx },
                marker: PipelineDataMarker(PhantomData),
            },
        )
    }
}

impl<P: Pipeline> VideoProcessor for PipelineHandler<P> {
    fn begin(&mut self) {
        // TODO: Move entity spawn logic from factory to here
        // TODO: Call smth on Pipeline
    }

    fn process<'a>(&'a mut self, img: &Mat) -> &'a Mat {
        let input = self.input.lock().expect("Lock input mutex").clone();
        self.pipeline.process(&self.output_tx, &*input, img)
    }

    fn end(&mut self) {
        let rst = self
            .output_tx
            .send(Box::new(|mut entity_world: EntityWorldMut| {
                P::cleanup(&mut entity_world);

                entity_world.despawn_recursive();
            }));
        if rst.is_err() {
            error!("Could not send cleanup callback to bevy");
        }
    }
}

#[derive(Bundle)]
pub struct PipelineBundle<P: Pipeline> {
    channels: PipelineChannels<P>,
    marker: PipelineDataMarker<P>,
}

#[derive(Component)]
struct PipelineDataMarker<P: Pipeline>(PhantomData<P>);

#[derive(Component)]
struct PipelineChannels<P: Pipeline> {
    input: ArcMutArc<P::Input>,
    output_rx: Receiver<EntityWorldCallback>,
}

pub fn pipeline_sync<P: Pipeline>(
    mut cmds: Commands,
    world: &World,
    query: Query<(EntityRef, &PipelineChannels<P>), With<PipelineDataMarker<P>>>,
) {
    for (entity, channels) in &query {
        // Forward new data from ECS
        let input = Arc::new(P::collect_inputs(world, &entity));
        if let Ok(mut lock) = channels.input.lock() {
            *lock = input;
        }

        // Schedule ECS write callbacks
        for callback in channels.output_rx.try_iter() {
            let entity = entity.id();
            cmds.add(move |world: &mut World| {
                let entity_world = world.entity_mut(entity);
                (callback)(entity_world);
            });
        }
    }
}
