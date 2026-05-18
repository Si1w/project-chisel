/// Marker trait for types that can be attached to an entity.
///
/// `'static` lets storage dispatch on `TypeId`; `Send + Sync` keeps
/// components movable across the async boundary even though v0 ticks
/// run single-threaded.
pub trait Component: 'static + Send + Sync {}
