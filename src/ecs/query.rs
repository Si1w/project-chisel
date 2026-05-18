use std::marker::PhantomData;

use crate::ecs::component::Component;
use crate::ecs::entity::Entity;
use crate::ecs::world::World;

/// What a query reads / writes. The associated `Item` is yielded per
/// iteration. Implemented for `&T`, `&mut T`, and tuples thereof.
pub trait QueryFetch: Sized {
    type Item<'w>;
}

/// Marker subtrait for fetches that only borrow components immutably.
/// `World::query` requires this bound so a `&self` access can't smuggle
/// out `&mut T` references.
pub trait ReadOnlyQuery: QueryFetch {}

impl<T: Component> QueryFetch for &T {
    type Item<'w> = &'w T;
}
impl<T: Component> ReadOnlyQuery for &T {}

impl<T: Component> QueryFetch for &mut T {
    type Item<'w> = &'w mut T;
}
// `&mut T` deliberately does NOT impl `ReadOnlyQuery`.

impl<A: QueryFetch, B: QueryFetch> QueryFetch for (A, B) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>);
}
impl<A: ReadOnlyQuery, B: ReadOnlyQuery> ReadOnlyQuery for (A, B) {}

impl<A: QueryFetch, B: QueryFetch, C: QueryFetch> QueryFetch for (A, B, C) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>, C::Item<'w>);
}
impl<A: ReadOnlyQuery, B: ReadOnlyQuery, C: ReadOnlyQuery> ReadOnlyQuery for (A, B, C) {}

impl<A: QueryFetch, B: QueryFetch, C: QueryFetch, D: QueryFetch> QueryFetch for (A, B, C, D) {
    type Item<'w> = (A::Item<'w>, B::Item<'w>, C::Item<'w>, D::Item<'w>);
}
impl<A: ReadOnlyQuery, B: ReadOnlyQuery, C: ReadOnlyQuery, D: ReadOnlyQuery> ReadOnlyQuery
    for (A, B, C, D)
{
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

impl<'w, Q: QueryFetch> QueryBuilder<'w, Q> {
    #[must_use]
    pub fn with<T: Component>(self) -> Self {
        todo!()
    }

    #[must_use]
    pub fn without<T: Component>(self) -> Self {
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
