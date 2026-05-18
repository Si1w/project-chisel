//! CLI-first 2D game engine for agent authoring.
//!
//! Module map mirrors the layer design in `.claude/design/architecture.md`.
//! Type signatures land here layer by layer; bodies are `todo!()` until
//! the corresponding step in the implementation plan.

pub mod cli;
pub mod component;
pub mod ecs;
pub mod event;
pub mod math;
pub mod physics;
pub mod runtime;
pub mod tag;
