use crate::ecs::component::Component;
use crate::ecs::entity::Entity;
use crate::ecs::error::Result;
use crate::ecs::query::{QueryBuilder, QueryFetch};
use crate::ecs::resource::Resource;
use crate::tag::registry::TagRegistry;

/// Owns all entity / component storage and the world-scoped resources
/// table. Not `Send` / `Sync` in v0 — engine ticks on a single thread.
#[derive(Default)]
pub struct World {
    // Archetype storage, entity allocator, and resource map. Private
    // until v0 implementation lands; outsiders go through the methods
    // below or via `Query` / resource accessors.
    _private: (),
}

impl World {
    /// Creates an empty world and auto-inserts the always-present
    /// resources (`TagRegistry`).
    #[must_use]
    pub fn new() -> Self {
        todo!()
    }

    // ---- entity lifecycle ----

    /// Allocates a new entity and returns a builder for attaching
    /// components before it becomes queryable.
    pub fn spawn(&mut self) -> EntityBuilder<'_> {
        todo!()
    }

    /// # Errors
    ///
    /// Returns `CoreError::EntityNotFound` if the entity is dead or its
    /// generation does not match.
    pub fn despawn(&mut self, _entity: Entity) -> Result<()> {
        todo!()
    }

    #[must_use]
    pub fn is_alive(&self, _entity: Entity) -> bool {
        todo!()
    }

    /// All currently live entities, in archetype-walk order.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        std::iter::empty()
    }

    // ---- per-entity component access ----

    #[must_use]
    pub fn get<T: Component>(&self, _entity: Entity) -> Option<&T> {
        todo!()
    }

    pub fn get_mut<T: Component>(&mut self, _entity: Entity) -> Option<&mut T> {
        todo!()
    }

    #[must_use]
    pub fn contains<T: Component>(&self, _entity: Entity) -> bool {
        todo!()
    }

    /// # Errors
    ///
    /// Returns `CoreError::EntityNotFound` if the entity is dead.
    pub fn insert<T: Component>(&mut self, _entity: Entity, _component: T) -> Result<()> {
        todo!()
    }

    /// # Errors
    ///
    /// Returns `CoreError::EntityNotFound` if the entity is dead, or
    /// `CoreError::ComponentMissing` if the component was not attached.
    pub fn remove<T: Component>(&mut self, _entity: Entity) -> Result<T> {
        todo!()
    }

    // ---- query ----

    pub fn query<Q: QueryFetch>(&self) -> QueryBuilder<'_, Q> {
        todo!()
    }

    // ---- resources ----

    pub fn insert_resource<R: Resource>(&mut self, _resource: R) {
        todo!()
    }

    #[must_use]
    pub fn resource<R: Resource>(&self) -> Option<&R> {
        todo!()
    }

    pub fn resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        todo!()
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        todo!()
    }

    #[must_use]
    pub fn contains_resource<R: Resource>(&self) -> bool {
        todo!()
    }

    /// Convenience accessor — `TagRegistry` is inserted by `World::new`
    /// and is always present.
    ///
    /// # Panics
    ///
    /// Panics if `TagRegistry` has been explicitly removed.
    #[must_use]
    pub fn tag_registry(&self) -> &TagRegistry {
        self.resource::<TagRegistry>()
            .expect("TagRegistry is inserted by World::new")
    }

    /// # Panics
    ///
    /// Panics if `TagRegistry` has been explicitly removed.
    pub fn tag_registry_mut(&mut self) -> &mut TagRegistry {
        self.resource_mut::<TagRegistry>()
            .expect("TagRegistry is inserted by World::new")
    }
}

/// Chained spawning. The entity is reserved up-front; calling `finish`
/// (or letting the builder drop) inserts it into the world.
pub struct EntityBuilder<'w> {
    _world: &'w mut World,
    _entity: Entity,
}

impl<'w> EntityBuilder<'w> {
    #[must_use]
    pub fn with<T: Component>(self, _component: T) -> Self {
        todo!()
    }

    pub fn finish(self) -> Entity {
        todo!()
    }
}
