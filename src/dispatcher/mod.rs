use lunaris_ecs::{bevy_ecs, prelude::*};

#[derive(Resource)]
pub struct DispatchReader {}

#[derive(Event)]
pub struct RenderRequest {
    entity: Entity,
}

pub struct RenderDag {
    head: RenderNode,
}

pub struct RenderNode {
    pub entity: Entity,
    pub children: Vec<Entity>,
}
