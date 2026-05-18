/// Marker trait for world-scoped singletons. One instance per `World`,
/// keyed by `TypeId`. Used for cross-system state that doesn't fit on
/// entities — physics globals (gravity), time, scoreboard, etc.
pub trait Resource: 'static + Send + Sync {}
