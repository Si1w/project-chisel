use serde::{Deserialize, Serialize};

use crate::ecs::resource::Resource;
use crate::math::vec2::Vec2;

/// World-scoped gravity vector applied by `AabbEngine` (and any future
/// `PhysicsEngine`). Defaults to zero so top-down games don't need
/// `[physics.gravity]` in their manifest.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Gravity(pub Vec2);

impl Resource for Gravity {}
