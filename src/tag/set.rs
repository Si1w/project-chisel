use crate::ecs::component::Component;
use crate::tag::id::TagId;

/// Set of `TagId`s attached to an entity. One `TagSet` component per
/// entity.
///
/// Internally a u128 inline bitset over `TagId`. `contains` is O(1) and
/// bulk filter operations (`contains_all`, `intersects`) are two-
/// instruction bitwise ops. v0 cap is 128 tag names per world; bumping
/// the cap means widening this inner type without touching callers.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct TagSet(u128);

impl Component for TagSet {}

impl TagSet {
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Adds `id`. Returns `true` if the tag was newly inserted.
    ///
    /// # Panics
    ///
    /// Panics if `id.0 >= 128`. Callers should obtain `TagId`s from
    /// `TagRegistry::intern`, which enforces the cap.
    pub fn insert(&mut self, id: TagId) -> bool {
        assert!(
            (id.0 as usize) < 128,
            "TagId {} exceeds u128 bitset cap",
            id.0
        );
        let mask = 1u128 << id.0;
        let was_set = (self.0 & mask) != 0;
        self.0 |= mask;
        !was_set
    }

    /// Removes `id`. Returns `true` if the tag was present.
    pub fn remove(&mut self, id: TagId) -> bool {
        if (id.0 as usize) >= 128 {
            return false;
        }
        let mask = 1u128 << id.0;
        let was_set = (self.0 & mask) != 0;
        self.0 &= !mask;
        was_set
    }

    #[must_use]
    pub fn contains(&self, id: TagId) -> bool {
        if (id.0 as usize) >= 128 {
            return false;
        }
        (self.0 >> id.0) & 1 != 0
    }

    /// `self ⊇ required` — every tag in `required` is also in `self`.
    #[must_use]
    pub fn contains_all(&self, required: &TagSet) -> bool {
        (self.0 & required.0) == required.0
    }

    /// `self ∩ other ≠ ∅` — at least one shared tag.
    #[must_use]
    pub fn intersects(&self, other: &TagSet) -> bool {
        (self.0 & other.0) != 0
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Iterator over all set `TagId`s in ascending order.
    #[must_use]
    pub fn iter(&self) -> TagSetIter {
        TagSetIter { bits: self.0 }
    }
}

impl IntoIterator for &TagSet {
    type Item = TagId;
    type IntoIter = TagSetIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct TagSetIter {
    bits: u128,
}

impl Iterator for TagSetIter {
    type Item = TagId;

    fn next(&mut self) -> Option<TagId> {
        if self.bits == 0 {
            return None;
        }
        let id =
            u16::try_from(self.bits.trailing_zeros()).expect("u128 bit index always fits in u16");
        self.bits &= self.bits - 1;
        Some(TagId(id))
    }
}
