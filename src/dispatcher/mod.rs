use bevy_ecs::{
    component::Component,
    entity::Entity,
    resource::Resource,
    system::{ObserverSystem, System},
};
use crossbeam::channel::Receiver;
use lunaris_api::{render::cache::TieredCache, timeline::elements::Properties};

#[derive(Resource)]
pub struct DispatchHandler {
    pub request: Receiver<Entity>,
    pub cache: TieredCache,
}

#[derive(Resource)]
pub struct DispatchReader {}

type RenderResourceJob = RenderResource<Box<dyn FnOnce() + Send + 'static>>;

#[derive(Component)]
pub struct RenderRequest {
    target: Entity,
}

pub struct RenderDag {
    layers: Vec<Vec<RenderResourceJob>>,
}

pub struct RenderResource<T>
where
    T: FnOnce() + Send + 'static,
{
    props: Properties,
    operation: T,
}
