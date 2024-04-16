pub mod edges;
pub mod marker;

use std::{
    borrow::Cow,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, bail};
use bevy::{
    app::{App, PluginGroup, PluginGroupBuilder, Update},
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, Resource},
        world::{EntityRef, EntityWorldMut, World},
    },
    hierarchy::DespawnRecursiveExt,
};
use common::error::ErrorEvent;
use crossbeam::{
    atomic::AtomicCell,
    channel::{bounded, Receiver, Sender},
};
use opencv::core::Mat;
use tracing::{debug, error};

use crate::{
    video_pipelines::{edges::EdgesPipelinePlugin, marker::MarkerPipelinePlugin},
    video_stream::{VideoProcessor, VideoProcessorFactory},
};

pub struct VideoPipelinePlugins;

impl PluginGroup for VideoPipelinePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(|app: &mut App| {
                let (cmd_tx, cmd_rx) = bounded(50);
                app.insert_resource(VideoCallbackChannels { cmd_tx, cmd_rx });
                app.add_systems(Update, schedule_pipeline_callbacks);
            })
            .add(EdgesPipelinePlugin)
            .add(MarkerPipelinePlugin)
    }
}

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
        self.add_systems(Update, forward_pipeline_inputs::<P>);

        self.init_resource::<VideoPipelines>();
        self.world
            .resource_mut::<VideoPipelines>()
            .0
            .push(VideoPipeline {
                name: P::NAME.into(),
                factory: factory::<P>(),
            });

        self
    }
}

#[derive(Resource)]
struct VideoCallbackChannels {
    cmd_tx: Sender<WorldCallback>,
    cmd_rx: Receiver<WorldCallback>,
}

#[derive(Resource, Default)]
pub struct VideoPipelines(pub Vec<VideoPipeline>);
pub struct VideoPipeline {
    pub name: Cow<'static, str>,
    pub factory: VideoProcessorFactory,
}

pub type WorldCallback = Box<dyn FnOnce(&mut World) + Send + Sync + 'static>;
pub type EntityWorldCallback = Box<dyn FnOnce(EntityWorldMut) + Send + Sync + 'static>;

pub trait Pipeline: Send + 'static {
    const NAME: &'static str;

    type Input: Default + Send + Sync + 'static;

    fn collect_inputs(world: &World, entity: &EntityRef) -> Self::Input;

    fn process<'b, 'a: 'b>(
        &'a mut self,
        cmds: PipelineCallbacks<'a>,
        data: &Self::Input,
        img: &'b mut Mat,
    ) -> anyhow::Result<&'b Mat>;

    /// Entity is implicitly despawned after this function returns
    fn cleanup(entity_world: &mut EntityWorldMut);
}

pub fn factory<P: Pipeline + Default>() -> VideoProcessorFactory {
    VideoProcessorFactory(P::NAME.into(), |world| {
        let channels = world.resource::<VideoCallbackChannels>();
        let cmds_tx = channels.cmd_tx.clone();

        Box::new(PipelineHandler::new(P::default(), cmds_tx))
    })
}

type ArcMutArc<T> = Arc<Mutex<Arc<T>>>;

pub struct PipelineHandler<P: Pipeline> {
    pipeline: P,

    entity: Arc<AtomicCell<Option<Entity>>>,
    bevy_handle: Arc<()>,
    input: ArcMutArc<P::Input>,
    cmds_tx: Sender<WorldCallback>,
    should_end: bool,
}

impl<P: Pipeline> PipelineHandler<P> {
    fn new(pipeline: P, cmds_tx: Sender<WorldCallback>) -> Self {
        let input: ArcMutArc<P::Input> = Default::default();

        Self {
            pipeline,
            entity: Arc::new(AtomicCell::new(None)),
            bevy_handle: Arc::new(()),
            input: input.clone(),
            cmds_tx,
            should_end: false,
        }
    }
}

impl<P: Pipeline> VideoProcessor for PipelineHandler<P> {
    fn begin(&mut self) {
        let refs = Arc::strong_count(&self.input);
        assert_eq!(
            refs,
            1,
            "PipelineHandler already has {} references",
            refs - 1
        );

        let entity = self.entity.clone();
        let input = self.input.clone();
        let bevy_handle = self.bevy_handle.clone();
        let res = self.cmds_tx.send(Box::new(move |world: &mut World| {
            let id = world
                .spawn(PipelineBundle::<P> {
                    channels: PipelineChannels { input },
                    marker: PipelineDataMarker(bevy_handle, PhantomData),
                })
                .id();

            entity.store(Some(id));
        }));

        if res.is_err() {
            error!("Could not send setup callback to bevy");
            self.should_end = true;
        }
    }

    fn process<'b, 'a: 'b>(&'a mut self, img: &'b mut Mat) -> anyhow::Result<&'b Mat> {
        let input = self.input.lock().expect("Lock input mutex").clone();
        let Some(entity) = self.entity.load() else {
            // self.should_end = true;

            bail!("PipelineHandler has no entity id");
        };

        let callbacks = PipelineCallbacks {
            cmds_tx: &self.cmds_tx,
            should_end: &mut self.should_end,
            entity,
        };

        self.pipeline.process(callbacks, &*input, img)
    }

    fn should_end(&self) -> bool {
        self.should_end || Arc::strong_count(&self.bevy_handle) == 1
    }

    fn end(&mut self) {
        let Some(entity) = self.entity.load() else {
            return;
        };

        let rst = self.cmds_tx.send(Box::new(move |world: &mut World| {
            let Some(mut entity_world) = world.get_entity_mut(entity) else {
                return;
            };

            P::cleanup(&mut entity_world);

            entity_world.despawn_recursive();
        }));

        if rst.is_err() {
            error!("Could not send cleanup callback to bevy");
        }
    }
}

pub struct PipelineCallbacks<'a> {
    cmds_tx: &'a Sender<WorldCallback>,
    should_end: &'a mut bool,
    entity: Entity,
}

impl PipelineCallbacks<'_> {
    pub fn world<F: FnOnce(&mut World) + Send + Sync + 'static>(&mut self, f: F) {
        let res = self.cmds_tx.send(Box::new(f));

        if res.is_err() {
            error!("Could not send world callback to bevy");
            *self.should_end = true;
        }
    }

    pub fn entity<F: FnOnce(EntityWorldMut) + Send + Sync + 'static>(&mut self, f: F) {
        let entity = self.entity;
        let res = self.cmds_tx.send(Box::new(move |world: &mut World| {
            let Some(entity) = world.get_entity_mut(entity) else {
                world.send_event(ErrorEvent(anyhow!("No entity for video entity callback")));

                return;
            };

            (f)(entity);
        }));

        if res.is_err() {
            error!("Could not send entity callback to bevy");
            *self.should_end = true;
        }
    }

    pub fn should_end(&mut self) {
        debug!("video pipeline should_end hit");
        *self.should_end = true;
    }
}

#[derive(Bundle)]
pub struct PipelineBundle<P: Pipeline> {
    channels: PipelineChannels<P>,
    marker: PipelineDataMarker<P>,
}

#[derive(Component)]
struct PipelineDataMarker<P: Pipeline>(Arc<()>, PhantomData<fn(P) -> P>);

#[derive(Component)]
struct PipelineChannels<P: Pipeline> {
    input: ArcMutArc<P::Input>,
}

fn schedule_pipeline_callbacks(mut cmds: Commands, channels: Res<VideoCallbackChannels>) {
    // Schedule ECS write callbacks
    for callback in channels.cmd_rx.try_iter() {
        cmds.add(callback);
    }
}

fn forward_pipeline_inputs<P: Pipeline>(
    world: &World,
    query: Query<(EntityRef, &PipelineChannels<P>), With<PipelineDataMarker<P>>>,
) {
    for (entity, channels) in &query {
        // Forward new data from ECS
        let input = Arc::new(P::collect_inputs(world, &entity));
        if let Ok(mut lock) = channels.input.lock() {
            *lock = input;
        }
    }
}
