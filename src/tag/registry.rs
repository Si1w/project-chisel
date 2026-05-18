use std::collections::HashMap;

use crate::ecs::error::Result;
use crate::ecs::resource::Resource;
use crate::tag::id::{TagId, MAX_TAGS};

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
    pub fn intern(&mut self, _name: &str) -> Result<TagId> {
        todo!()
    }

    #[must_use]
    pub fn lookup(&self, _name: &str) -> Option<TagId> {
        todo!()
    }

    #[must_use]
    pub fn name(&self, _id: TagId) -> Option<&str> {
        todo!()
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
