use serde::{Deserialize, Serialize};

use crate::ecs::component::Component;

/// Animation clip identifier plus metadata. v0 only carries the name;
/// later versions add intrinsic length, marker offsets, asset handle,
/// blend weights, etc. without touching `Animator`'s shape.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Clip {
    pub name: String,
}

/// v0 animation playback state. The animation system only advances
/// `elapsed`; real sampling lands when `Clip` grows actual data.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Animator {
    pub clip: Clip,
    pub elapsed: f32,
    pub speed: f32,
    pub looping: bool,
}
impl Component for Animator {}
