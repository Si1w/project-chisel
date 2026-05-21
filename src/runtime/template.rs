use std::collections::HashMap;

use crate::component::animation::Animator;
use crate::component::spatial::{Aabb, Position, Velocity};
use crate::ecs::entity::Entity;
use crate::ecs::error::Result;
use crate::ecs::resource::Resource;
use crate::ecs::world::World;
use crate::tag::set::TagSet;

/// One instantiable entity template, loaded from `entities/*.toml`.
/// Tag names have been resolved to interned `TagId`s and packed into
/// `TagSet` at load time, so spawning is a flat clone-and-attach.
///
/// v0 carries the closed component set inline; a future dynamic
/// component path will replace these fields with reflected storage.
#[derive(Debug, Clone, Default)]
pub struct Template {
    pub name: String,
    pub tags: TagSet,
    pub position: Option<Position>,
    pub velocity: Option<Velocity>,
    pub aabb: Option<Aabb>,
    pub animator: Option<Animator>,
}

/// World-scoped registry of entity templates. Inserted by `bootstrap`
/// after parsing `entities/*.toml`; consumed by the `spawn` rule
/// action.
#[derive(Debug, Default)]
pub struct TemplateStore {
    by_name: HashMap<String, Template>,
}

impl Resource for TemplateStore {}

impl TemplateStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, template: Template) {
        self.by_name.insert(template.name.clone(), template);
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Template> {
        self.by_name.get(name)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Spawns one entity from an already-resolved template.
///
/// # Errors
///
/// Returns `CoreError::EntityNotFound` if the newly allocated entity
/// cannot receive components.
pub(crate) fn spawn_template(world: &mut World, template: Template) -> Result<Entity> {
    let entity = world.spawn().finish();
    world.insert(entity, template.tags)?;
    if let Some(component) = template.position {
        world.insert(entity, component)?;
    }
    if let Some(component) = template.velocity {
        world.insert(entity, component)?;
    }
    if let Some(component) = template.aabb {
        world.insert(entity, component)?;
    }
    if let Some(component) = template.animator {
        world.insert(entity, component)?;
    }

    Ok(entity)
}
