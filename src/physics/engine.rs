use crate::ecs::schedule::System;

/// Marker trait for "this system runs the physics step." v0 has
/// `AabbEngine`; v2+ may add `Rapier2dEngine`. Body intentionally empty
/// until a second implementation reveals a real common surface.
pub trait PhysicsEngine: System + Send {}
