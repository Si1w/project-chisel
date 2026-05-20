use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;

use crate::ecs::component::Component;
use crate::ecs::entity::Entity;
use crate::ecs::error::{CoreError, Result};
use crate::ecs::query::{QueryBuilder, QueryFetch, QueryMutBuilder};
use crate::ecs::resource::Resource;
use crate::event::queue::EventQueue;
use crate::tag::registry::TagRegistry;

pub(crate) type ComponentBox = Box<dyn Any + Send + Sync>;
pub(crate) type ComponentStore = HashMap<Entity, ComponentBox>;
type ResourceBox = Box<dyn Any + Send + Sync>;

/// Owns all entity / component storage and the world-scoped resources
/// table. v0 engine ticks on a single thread.
pub struct World {
    generations: Vec<u32>,
    alive: Vec<bool>,
    free: Vec<u32>,
    components: HashMap<TypeId, ComponentStore>,
    resources: HashMap<TypeId, ResourceBox>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Creates an empty world and auto-inserts the always-present
    /// resources: `TagRegistry`, `EventQueue`.
    #[must_use]
    pub fn new() -> Self {
        let mut world = Self {
            generations: Vec::new(),
            alive: Vec::new(),
            free: Vec::new(),
            components: HashMap::new(),
            resources: HashMap::new(),
        };
        world.insert_resource(TagRegistry::new());
        world.insert_resource(EventQueue::new());
        world
    }

    // ---- entity lifecycle ----

    /// Allocates a new entity and returns a builder for attaching
    /// components before it becomes queryable.
    ///
    /// # Panics
    ///
    /// Panics if more than `u32::MAX` entity slots are allocated.
    pub fn spawn(&mut self) -> EntityBuilder<'_> {
        let index = if let Some(index) = self.free.pop() {
            index
        } else {
            let index =
                u32::try_from(self.generations.len()).expect("entity index space exhausted");
            self.generations.push(0);
            self.alive.push(false);
            index
        };
        let slot = usize::try_from(index).expect("entity index fits usize");
        self.alive[slot] = true;
        let entity = Entity {
            index,
            generation: self.generations[slot],
        };
        EntityBuilder {
            world: self,
            entity,
        }
    }

    /// # Errors
    ///
    /// Returns `CoreError::EntityNotFound` if the entity is dead or its
    /// generation does not match.
    pub fn despawn(&mut self, entity: Entity) -> Result<()> {
        let index = self
            .entity_index(entity)
            .ok_or(CoreError::EntityNotFound(entity))?;
        self.alive[index] = false;
        self.generations[index] = self.generations[index].saturating_add(1);
        self.free.push(entity.index);
        for store in self.components.values_mut() {
            store.remove(&entity);
        }
        Ok(())
    }

    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entity_index(entity).is_some()
    }

    /// All currently live entities, in archetype-walk order.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .filter_map(|(index, _)| {
                Some(Entity {
                    index: u32::try_from(index).ok()?,
                    generation: self.generations[index],
                })
            })
    }

    // ---- per-entity component access ----

    #[must_use]
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }
        self.components
            .get(&TypeId::of::<T>())?
            .get(&entity)?
            .downcast_ref()
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }
        self.components
            .get_mut(&TypeId::of::<T>())?
            .get_mut(&entity)?
            .downcast_mut()
    }

    #[must_use]
    pub fn contains<T: Component>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// # Errors
    ///
    /// Returns `CoreError::EntityNotFound` if the entity is dead.
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) -> Result<()> {
        if !self.is_alive(entity) {
            return Err(CoreError::EntityNotFound(entity));
        }
        self.insert_component(entity, component);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns `CoreError::EntityNotFound` if the entity is dead, or
    /// `CoreError::ComponentMissing` if the component was not attached.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Result<T> {
        if !self.is_alive(entity) {
            return Err(CoreError::EntityNotFound(entity));
        }
        let store = self
            .components
            .get_mut(&TypeId::of::<T>())
            .ok_or_else(|| component_missing::<T>(entity))?;
        let component = store
            .remove(&entity)
            .ok_or_else(|| component_missing::<T>(entity))?;
        match component.downcast::<T>() {
            Ok(component) => Ok(*component),
            Err(_) => Err(component_missing::<T>(entity)),
        }
    }

    // ---- query ----

    /// Read-only query. `Q` is composed of `&T` fetches only.
    #[must_use]
    pub fn query<Q: QueryFetch>(&self) -> QueryBuilder<'_, Q> {
        QueryBuilder::new(self)
    }

    /// Mutable query for one component type. Takes `&mut self` so yielded
    /// component borrows are tied to exclusive world access.
    #[must_use]
    pub fn query_mut<T: Component>(&mut self) -> QueryMutBuilder<'_, T> {
        QueryMutBuilder::new(self)
    }

    // ---- resources ----

    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(resource));
    }

    #[must_use]
    pub fn resource<R: Resource>(&self) -> Option<&R> {
        self.resources.get(&TypeId::of::<R>())?.downcast_ref()
    }

    pub fn resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.resources.get_mut(&TypeId::of::<R>())?.downcast_mut()
    }

    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        let resource = self.resources.remove(&TypeId::of::<R>())?;
        resource.downcast::<R>().ok().map(|resource| *resource)
    }

    #[must_use]
    pub fn contains_resource<R: Resource>(&self) -> bool {
        self.resource::<R>().is_some()
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

    fn entity_index(&self, entity: Entity) -> Option<usize> {
        let index = usize::try_from(entity.index).ok()?;
        if self.alive.get(index).copied()? && self.generations[index] == entity.generation {
            Some(index)
        } else {
            None
        }
    }

    fn insert_component<T: Component>(&mut self, entity: Entity, component: T) {
        self.components
            .entry(TypeId::of::<T>())
            .or_default()
            .insert(entity, Box::new(component));
    }

    pub(crate) fn contains_type_id(&self, entity: Entity, type_id: TypeId) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        self.components
            .get(&type_id)
            .is_some_and(|store| store.contains_key(&entity))
    }

    pub(crate) fn matching_entities(
        &self,
        required: &[TypeId],
        forbidden: &[TypeId],
    ) -> Vec<Entity> {
        self.entities()
            .filter(|entity| {
                required
                    .iter()
                    .all(|type_id| self.contains_type_id(*entity, *type_id))
                    && forbidden
                        .iter()
                        .all(|type_id| !self.contains_type_id(*entity, *type_id))
            })
            .collect()
    }

    pub(crate) fn component_store_mut<T: Component>(&mut self) -> Option<&mut ComponentStore> {
        self.components.get_mut(&TypeId::of::<T>())
    }
}

/// Chained spawning. The entity is reserved up-front; calling `finish`
/// (or letting the builder drop) inserts it into the world.
pub struct EntityBuilder<'w> {
    world: &'w mut World,
    entity: Entity,
}

impl EntityBuilder<'_> {
    #[must_use]
    pub fn with<T: Component>(self, component: T) -> Self {
        self.world.insert_component(self.entity, component);
        self
    }

    #[must_use]
    pub fn finish(self) -> Entity {
        self.entity
    }
}

fn component_missing<T: Component>(entity: Entity) -> CoreError {
    CoreError::ComponentMissing {
        entity,
        component: type_name::<T>(),
    }
}

#[cfg(test)]
mod tests {
    use crate::component::spatial::{Aabb, Position, Velocity};
    use crate::ecs::error::CoreError;
    use crate::event::queue::EventQueue;
    use crate::math::vec2::Vec2;

    use super::*;

    #[derive(Debug, Eq, PartialEq)]
    struct Score(u32);
    impl Resource for Score {}

    #[test]
    fn new_world_contains_core_resources() {
        let world = World::new();

        assert!(world.contains_resource::<TagRegistry>());
        assert!(world.contains_resource::<EventQueue>());
        assert!(world.tag_registry().is_empty());
    }

    #[test]
    fn resources_round_trip_by_type() {
        let mut world = World::new();

        world.insert_resource(Score(7));
        assert_eq!(world.resource::<Score>(), Some(&Score(7)));

        world.resource_mut::<Score>().expect("score exists").0 += 1;
        assert_eq!(world.remove_resource::<Score>(), Some(Score(8)));
        assert!(!world.contains_resource::<Score>());
    }

    #[test]
    fn spawn_attaches_components_and_allows_mutation() {
        let mut world = World::new();

        let entity = world
            .spawn()
            .with(Position(Vec2::new(1.0, 2.0)))
            .with(Velocity(Vec2::new(3.0, 4.0)))
            .finish();

        assert!(world.is_alive(entity));
        assert_eq!(world.entities().collect::<Vec<_>>(), vec![entity]);
        assert_eq!(
            world.get::<Position>(entity).map(|p| p.0),
            Some(Vec2::new(1.0, 2.0))
        );
        assert!(world.contains::<Velocity>(entity));

        world
            .get_mut::<Position>(entity)
            .expect("position exists")
            .x = 5.0;
        assert_eq!(world.get::<Position>(entity).map(|p| p.x), Some(5.0));
    }

    #[test]
    fn remove_and_despawn_clear_component_access() {
        let mut world = World::new();
        let entity = world.spawn().with(Position(Vec2::ZERO)).finish();

        let removed = world.remove::<Position>(entity).expect("position exists");
        assert_eq!(removed.0, Vec2::ZERO);
        assert!(matches!(
            world.remove::<Position>(entity),
            Err(CoreError::ComponentMissing { entity: missing_entity, .. })
                if missing_entity == entity
        ));

        world
            .insert(entity, Position(Vec2::new(9.0, 0.0)))
            .expect("entity is alive");
        world.despawn(entity).expect("entity is alive");

        assert!(!world.is_alive(entity));
        assert_eq!(world.get::<Position>(entity).map(|p| p.0), None);
        assert!(matches!(
            world.insert(entity, Position(Vec2::ZERO)),
            Err(CoreError::EntityNotFound(missing_entity)) if missing_entity == entity
        ));
    }

    #[test]
    fn despawn_reuses_index_with_new_generation() {
        let mut world = World::new();

        let first = world.spawn().finish();
        world.despawn(first).expect("entity is alive");
        let second = world.spawn().finish();

        assert_eq!(first.index, second.index);
        assert_ne!(first.generation, second.generation);
        assert!(!world.is_alive(first));
        assert!(world.is_alive(second));
    }

    #[test]
    fn query_returns_entities_with_requested_components() {
        let mut world = World::new();
        let moving = world
            .spawn()
            .with(Position(Vec2::new(1.0, 0.0)))
            .with(Velocity(Vec2::new(2.0, 0.0)))
            .finish();
        let _ = world.spawn().with(Position(Vec2::new(9.0, 0.0))).finish();

        let rows = world
            .query::<(&Position, &Velocity)>()
            .into_iter()
            .map(|(entity, (position, velocity))| (entity, position.0, velocity.0))
            .collect::<Vec<_>>();

        assert_eq!(
            rows,
            vec![(moving, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0))]
        );
    }

    #[test]
    fn query_filters_with_and_without_components() {
        let mut world = World::new();
        let expected = world
            .spawn()
            .with(Position(Vec2::new(1.0, 0.0)))
            .with(Velocity(Vec2::new(1.0, 0.0)))
            .finish();
        let _ = world.spawn().with(Position(Vec2::new(2.0, 0.0))).finish();
        let _ = world
            .spawn()
            .with(Position(Vec2::new(3.0, 0.0)))
            .with(Velocity(Vec2::new(3.0, 0.0)))
            .with(Aabb {
                half_extents: Vec2::new(1.0, 1.0),
            })
            .finish();

        let entities = world
            .query::<&Position>()
            .with::<Velocity>()
            .without::<Aabb>()
            .into_iter()
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();

        assert_eq!(entities, vec![expected]);
    }

    #[test]
    fn query_preserves_entity_iteration_order() {
        let mut world = World::new();
        let first = world.spawn().with(Position(Vec2::new(1.0, 0.0))).finish();
        let second = world.spawn().with(Position(Vec2::new(2.0, 0.0))).finish();

        let entities = world
            .query::<&Position>()
            .into_iter()
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();

        assert_eq!(entities, vec![first, second]);
    }

    #[test]
    fn query_mut_updates_single_component_with_filters() {
        let mut world = World::new();
        let still = world.spawn().with(Position(Vec2::new(1.0, 0.0))).finish();
        let moving = world
            .spawn()
            .with(Position(Vec2::new(2.0, 0.0)))
            .with(Velocity(Vec2::new(1.0, 0.0)))
            .finish();

        for (_, position) in world.query_mut::<Position>().with::<Velocity>() {
            position.x += 10.0;
        }

        assert_eq!(world.get::<Position>(still).map(|p| p.x), Some(1.0));
        assert_eq!(world.get::<Position>(moving).map(|p| p.x), Some(12.0));
    }
}
