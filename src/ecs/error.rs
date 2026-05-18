use crate::ecs::entity::Entity;

/// Errors returned by ECS operations that callers may pattern-match.
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("entity {0:?} not found or generation mismatch")]
    EntityNotFound(Entity),

    #[error("component {component} missing on entity {entity:?}")]
    ComponentMissing {
        entity: Entity,
        component: &'static str,
    },

    #[error("tag namespace full ({0} interned, cap is 128)")]
    TagsFull(usize),
}

pub type Result<T> = std::result::Result<T, CoreError>;
