use serde::{Deserialize, Serialize};

use crate::ecs::component::Component;
use crate::math::vec2::Vec2;

/// World-space position of an entity. Read and written by the physics
/// integrate step.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Position(pub Vec2);
impl Component for Position {}

impl std::ops::Deref for Position {
    type Target = Vec2;
    fn deref(&self) -> &Vec2 {
        &self.0
    }
}

impl std::ops::DerefMut for Position {
    fn deref_mut(&mut self) -> &mut Vec2 {
        &mut self.0
    }
}

/// World-space velocity. Added to `Position` each tick (scaled by `dt`).
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Velocity(pub Vec2);
impl Component for Velocity {}

impl std::ops::Deref for Velocity {
    type Target = Vec2;
    fn deref(&self) -> &Vec2 {
        &self.0
    }
}

impl std::ops::DerefMut for Velocity {
    fn deref_mut(&mut self) -> &mut Vec2 {
        &mut self.0
    }
}

/// Axis-aligned bounding box. The full box spans
/// `[Position - half_extents, Position + half_extents]`.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Aabb {
    pub half_extents: Vec2,
}
impl Component for Aabb {}
