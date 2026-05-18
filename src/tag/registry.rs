use std::collections::HashMap;

use crate::ecs::error::{CoreError, Result};
use crate::ecs::resource::Resource;
use crate::tag::id::{MAX_TAGS, TagId};

/// World-scoped intern table for tag names. One per `World`; lives as a
/// `Resource` so any system can borrow it through the standard ECS API.
#[derive(Debug, Default)]
pub struct TagRegistry {
    by_name: HashMap<String, TagId>,
    by_id: Vec<String>,
}

impl Resource for TagRegistry {}

impl TagRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get-or-insert. Returns the same `TagId` for the same name across
    /// the lifetime of the `World`.
    ///
    /// # Errors
    ///
    /// Returns `CoreError::TagsFull` once `MAX_TAGS` distinct names have
    /// been interned in this registry.
    pub fn intern(&mut self, name: &str) -> Result<TagId> {
        if let Some(id) = self.by_name.get(name).copied() {
            return Ok(id);
        }

        if self.is_full() {
            return Err(CoreError::TagsFull(self.by_id.len()));
        }

        let id = TagId(
            u16::try_from(self.by_id.len()).map_err(|_| CoreError::TagsFull(self.by_id.len()))?,
        );
        self.by_id.push(name.to_owned());
        self.by_name.insert(name.to_owned(), id);
        Ok(id)
    }

    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<TagId> {
        self.by_name.get(name).copied()
    }

    #[must_use]
    pub fn name(&self, id: TagId) -> Option<&str> {
        self.by_id.get(id.0 as usize).map(String::as_str)
    }

    /// Current number of distinct interned tags.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// `true` once `intern` would start returning `TagsFull`.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.by_id.len() >= MAX_TAGS
    }
}

#[cfg(test)]
mod tests {
    use crate::ecs::error::CoreError;

    use super::*;

    #[test]
    fn intern_returns_stable_id_and_round_trips_name() {
        let mut registry = TagRegistry::new();

        let first = registry.intern("Player").expect("tag should intern");
        let second = registry.intern("Player").expect("tag should intern again");

        assert_eq!(first, second);
        assert_eq!(registry.lookup("Player"), Some(first));
        assert_eq!(registry.name(first), Some("Player"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn intern_rejects_names_after_bitset_capacity() {
        let mut registry = TagRegistry::new();

        for index in 0..MAX_TAGS {
            registry
                .intern(&format!("Tag{index}"))
                .expect("tag should fit before capacity");
        }

        let error = registry
            .intern("TooMany")
            .expect_err("tag namespace should be full");

        match error {
            CoreError::TagsFull(count) => assert_eq!(count, MAX_TAGS),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
