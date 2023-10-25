use crate::{adapters, generate_adapters_components, generate_adapters_resources};

generate_adapters_components! {
    name = adapters_components,
    output = adapters::BackingType,
    tokens = {

    }
}
generate_adapters_resources! {
    name = adapters_resources,
    output = adapters::BackingType,
    tokens = {

    }
}
