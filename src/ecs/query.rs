use std::marker::PhantomData;

use crate::ecs::component::Component;
use crate::ecs::entity::Entity;
use crate::ecs::world::World;

/// What a read-only query fetches. The associated `Item` is yielded per
/// iteration. Implemented for `&T` and tuples thereof.
pub trait QueryFetch: Sized {
    type Item<'w>;
}

impl<T: Component> QueryFetch for &T {
    type Item<'w> = &'w T;
}

impl<A: QueryFetch, B: QueryFetch> QueryFetch for (A, B) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>);
}

impl<A: QueryFetch, B: QueryFetch, C: QueryFetch> QueryFetch for (A, B, C) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>, C::Item<'w>);
}

impl<A: QueryFetch, B: QueryFetch, C: QueryFetch, D: QueryFetch> QueryFetch for (A, B, C, D) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>, C::Item<'w>, D::Item<'w>);
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
    _world: &'w World,
    _q: PhantomData<Q>,
}

impl<Q: QueryFetch> QueryBuilder<'_, Q> {
    #[must_use]
    pub fn with<T: Component>(self) -> Self {
        todo!()
    }

    #[must_use]
    pub fn without<T: Component>(self) -> Self {
        todo!()
    }
}

/// Mutable query for one component type. v0 intentionally avoids tuple
/// mutable queries until storage can prove disjoint component borrows.
pub struct QueryMutBuilder<'w, T: Component> {
    _world: &'w mut World,
    _component: PhantomData<T>,
}

impl<T: Component> QueryMutBuilder<'_, T> {
    #[must_use]
    pub fn with<U: Component>(self) -> Self {
        todo!()
    }

    #[must_use]
    pub fn without<U: Component>(self) -> Self {
        todo!()
    }
}

impl<'w, T: Component + 'w> IntoIterator for QueryMutBuilder<'w, T> {
    type Item = (Entity, &'w mut T);
    type IntoIter = QueryMutIter<'w, T>;

    fn into_iter(self) -> QueryMutIter<'w, T> {
        todo!()
    }
}

pub struct QueryMutIter<'w, T: Component> {
    _phantom: PhantomData<(&'w mut (), T)>,
}

impl<'w, T: Component + 'w> Iterator for QueryMutIter<'w, T> {
    type Item = (Entity, &'w mut T);

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl<'w, Q: QueryFetch + 'w> IntoIterator for QueryBuilder<'w, Q> {
    type Item = (Entity, Q::Item<'w>);
    type IntoIter = QueryIter<'w, Q>;

    fn into_iter(self) -> QueryIter<'w, Q> {
        todo!()
    }
}

pub struct QueryIter<'w, Q: QueryFetch> {
    _phantom: PhantomData<(&'w (), Q)>,
}

impl<'w, Q: QueryFetch + 'w> Iterator for QueryIter<'w, Q> {
    type Item = (Entity, Q::Item<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
