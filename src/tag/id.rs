use serde::{Deserialize, Serialize};

/// Interned identifier for a tag name. Cheap to copy and compare; the
/// mapping back to `&str` lives in `TagRegistry`.
///
/// Even though the storage is `u16`, the effective cap is 128 because
/// `TagSet` is a u128 inline bitset (v0). `TagRegistry::intern` returns
/// `Err` once the cap is reached.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TagId(pub u16);

/// Maximum number of distinct tag names a single `World` can intern in
/// v0. Bumping this requires widening `TagSet`'s inner bitset.
pub const MAX_TAGS: usize = 128;
