use std::any::TypeId;
use std::marker::PhantomData;
use std::vec::IntoIter;

use crate::ecs::component::Component;
use crate::ecs::entity::Entity;
use crate::ecs::world::World;

/// What a read-only query fetches. The associated `Item` is yielded per
/// iteration. Implemented for `&T` and tuples thereof.
pub trait QueryFetch: Sized {
    type Item<'w>;

    fn fetch(world: &World, entity: Entity) -> Option<Self::Item<'_>>;
}

impl<T: Component> QueryFetch for &T {
    type Item<'w> = &'w T;

    fn fetch(world: &World, entity: Entity) -> Option<Self::Item<'_>> {
        world.get::<T>(entity)
    }
}

impl<A: QueryFetch, B: QueryFetch> QueryFetch for (A, B) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>);

    fn fetch(world: &World, entity: Entity) -> Option<Self::Item<'_>> {
        Some((A::fetch(world, entity)?, B::fetch(world, entity)?))
    }
}

impl<A: QueryFetch, B: QueryFetch, C: QueryFetch> QueryFetch for (A, B, C) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>, C::Item<'w>);

    fn fetch(world: &World, entity: Entity) -> Option<Self::Item<'_>> {
        Some((
            A::fetch(world, entity)?,
            B::fetch(world, entity)?,
            C::fetch(world, entity)?,
        ))
    }
}

impl<A: QueryFetch, B: QueryFetch, C: QueryFetch, D: QueryFetch> QueryFetch for (A, B, C, D) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>, C::Item<'w>, D::Item<'w>);

    fn fetch(world: &World, entity: Entity) -> Option<Self::Item<'_>> {
        Some((
            A::fetch(world, entity)?,
            B::fetch(world, entity)?,
            C::fetch(world, entity)?,
            D::fetch(world, entity)?,
        ))
    }
}

/// Type-level marker: entity must have component `T` but the query does
/// not borrow it. Used via `QueryBuilder::with<T>` for clarity in v0.
pub struct With<T: Component>(PhantomData<T>);

/// Type-level marker: entity must not have component `T`.
pub struct Without<T: Component>(PhantomData<T>);

/// Builder returned by `World::query`. Filters chain via `.with` /
/// `.without`. Drive iteration via a `for` loop (it implements
/// `IntoIterator`).
pub struct QueryBuilder<'w, Q: QueryFetch> {
    world: &'w World,
    required: Vec<TypeId>,
    forbidden: Vec<TypeId>,
    _q: PhantomData<Q>,
}

impl<Q: QueryFetch> QueryBuilder<'_, Q> {
    #[must_use]
    pub fn with<T: Component>(self) -> Self {
        self.with_type(TypeId::of::<T>())
    }

    #[must_use]
    pub fn without<T: Component>(self) -> Self {
        self.without_type(TypeId::of::<T>())
    }
}

impl<'w, Q: QueryFetch> QueryBuilder<'w, Q> {
    #[must_use]
    pub(crate) fn new(world: &'w World) -> Self {
        Self {
            world,
            required: Vec::new(),
            forbidden: Vec::new(),
            _q: PhantomData,
        }
    }

    #[must_use]
    fn with_type(mut self, type_id: TypeId) -> Self {
        self.required.push(type_id);
        self
    }

    #[must_use]
    fn without_type(mut self, type_id: TypeId) -> Self {
        self.forbidden.push(type_id);
        self
    }
}

/// Mutable query for one component type. v0 intentionally avoids tuple
/// mutable queries until storage can prove disjoint component borrows.
pub struct QueryMutBuilder<'w, T: Component> {
    world: &'w mut World,
    required: Vec<TypeId>,
    forbidden: Vec<TypeId>,
    _component: PhantomData<T>,
}

impl<T: Component> QueryMutBuilder<'_, T> {
    #[must_use]
    pub fn with<U: Component>(self) -> Self {
        self.with_type(TypeId::of::<U>())
    }

    #[must_use]
    pub fn without<U: Component>(self) -> Self {
        self.without_type(TypeId::of::<U>())
    }
}

impl<'w, T: Component> QueryMutBuilder<'w, T> {
    #[must_use]
    pub(crate) fn new(world: &'w mut World) -> Self {
        Self {
            world,
            required: Vec::new(),
            forbidden: Vec::new(),
            _component: PhantomData,
        }
    }

    #[must_use]
    fn with_type(mut self, type_id: TypeId) -> Self {
        self.required.push(type_id);
        self
    }

    #[must_use]
    fn without_type(mut self, type_id: TypeId) -> Self {
        self.forbidden.push(type_id);
        self
    }
}

impl<'w, T: Component + 'w> IntoIterator for QueryMutBuilder<'w, T> {
    type Item = (Entity, &'w mut T);
    type IntoIter = QueryMutIter<'w, T>;

    fn into_iter(self) -> QueryMutIter<'w, T> {
        let matches = self
            .world
            .matching_entities(&self.required, &self.forbidden);
        let rows = self
            .world
            .component_store_mut::<T>()
            .map(|store| {
                store
                    .iter_mut()
                    .filter(|(entity, _)| matches.contains(entity))
                    .filter_map(|(entity, component)| {
                        component
                            .downcast_mut::<T>()
                            .map(|component| (*entity, component))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        QueryMutIter {
            inner: rows.into_iter(),
        }
    }
}

pub struct QueryMutIter<'w, T: Component> {
    inner: IntoIter<(Entity, &'w mut T)>,
}

impl<'w, T: Component + 'w> Iterator for QueryMutIter<'w, T> {
    type Item = (Entity, &'w mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'w, Q: QueryFetch + 'w> IntoIterator for QueryBuilder<'w, Q> {
    type Item = (Entity, Q::Item<'w>);
    type IntoIter = QueryIter<'w, Q>;

    fn into_iter(self) -> QueryIter<'w, Q> {
        let rows = self
            .world
            .matching_entities(&self.required, &self.forbidden)
            .into_iter()
            .filter_map(|entity| Q::fetch(self.world, entity).map(|item| (entity, item)))
            .collect::<Vec<_>>();

        QueryIter {
            inner: rows.into_iter(),
        }
    }
}

pub struct QueryIter<'w, Q: QueryFetch> {
    inner: IntoIter<(Entity, Q::Item<'w>)>,
}

impl<'w, Q: QueryFetch + 'w> Iterator for QueryIter<'w, Q> {
    type Item = (Entity, Q::Item<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
