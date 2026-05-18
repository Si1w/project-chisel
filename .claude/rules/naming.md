# Naming Conventions

Read this before introducing or changing project names, domain terms, public
identifiers, file names, config keys, CLI names, error names, or user-facing
labels.

These rules summarize local naming decisions for this repository. They
complement `.claude/rules/code-style.md`; when in doubt, follow the existing
module shape and keep the naming table current.

## Feature Work Requirement

- Before designing or implementing a feature, review this file.
- During design, identify every new project concept, module boundary, public
  type, public function, file name, config key, CLI name, error name, and
  user-facing label the feature introduces.
- If a concept is missing from the naming table, add it before using the name in
  code, docs, tests, configuration, or UI text.
- If a concept is already in the table, use the canonical term and allowed forms
  exactly as documented.

## Rust Names

- Use Rust-standard casing: modules, functions, methods, and locals in
  `snake_case`; types, traits, and enum variants in `UpperCamelCase`; constants
  and statics in `SCREAMING_SNAKE_CASE`.
- Treat acronyms as words in new `UpperCamelCase` names: use `Uuid`, not `UUID`.
- Prefer precise names over short vague names. Small local variables can be
  short; public types and cross-module APIs should say what they represent.
- Apply Rust conversion naming rules from `.claude/rules/code-style.md` after
  choosing the canonical term from this file.

## Module Boundaries

- Top-level modules should be bounded-context nouns.
- Inside a bounded module, avoid repeating the module name when the path already
  carries the context. Prefer `domain::Store` over `DomainStore` unless the type
  is commonly used outside that module and needs context.
- Use re-export aliases sparingly when they improve caller clarity without
  changing the underlying module ownership.

## Naming Table Rules

- Use the canonical term from the naming table whenever the concept already
  exists.
- If a concept is not in the table, add it to the table before using the new
  name in code, docs, tests, configuration, or UI text.
- Do not introduce synonyms, abbreviations, or alternate spellings unless the
  table explicitly allows them.
- Keep names self-documenting. Prefer precise names over short vague names.

## Naming Table

Three sub-tables. **Add new names by extending Suffix / Verb tables first, then
fall back to Concept-keyed reservations only for names that cannot be derived
from a pattern.**

### Suffix Conventions

| Suffix | Meaning | Examples |
| --- | --- | --- |
| `*Id` | Interned handle (`u16` / `u32`); cheap copy; round-trip via a `*Registry` | `TagId`; reserved: `ClipId`, `RuleId` |
| `*Registry` | World-scoped name ↔ id intern table | `TagRegistry`; reserved: `ClipRegistry` |
| `*Set` | Set-semantic collection with bulk ops (`contains_all`, `intersects`) | `TagSet`; reserved: `ChannelSet` |
| `*Engine` | Pluggable trait + impl pair | `PhysicsEngine` / `AabbEngine`; reserved: `RenderEngine` |
| `*Tx` / `*Rx` | Async channel sender / receiver wrapper | `BusTx`, `BusRx` |
| `*Event` | Payload type traveling on the bus | `DomainEvent`, `MarkerEvent`, `InputEvent` |
| `*Command` | Control-plane or presentation directive | `PresentationCommand` |
| `*Error` | Per-crate error enum (`thiserror`) | `CoreError`, `PhysicsError`, `RuntimeError` |
| `*Schema` | Manifest schema struct (`serde::Deserialize` target); format-agnostic | `GameSchema`, `RuleSchema`, `InputSchema` |

### Verb Conventions

| Verb | Use | Avoid |
| --- | --- | --- |
| `intern` | get-or-insert in a `*Registry` | `register`, `get_or_create`, `add` |
| `lookup` | read-only query by string in a `*Registry` | `find`, `get_by_name`, `resolve` |
| `name` | id → `Option<&str>` in a `*Registry` | `name_of`, `to_string`, `display` |
| `spawn` / `despawn` | create / remove an entity | `create` / `delete`, `kill`, `destroy` |
| `get` / `get_mut` | borrow a component from an entity | `fetch`, `read` |
| `query` | build an iterator over entities matching a filter | `find`, `search`, `select` |
| `insert` / `remove` | mutate a set or map | `add` / `delete` |
| `contains` / `contains_all` / `intersects` | set membership / superset / overlap | `has`, `includes`, `overlaps`, `is_superset` |
| `publish` | send an event to the bus (channel-specific, low-level) | `send`, `fire`, `post` |
| `subscribe` | get a channel-specific `*Rx` | `listen`, `on`, `connect` |
| `emit` | rule/system-side alias for `publish` on `domain` / `presentation` channels | `send`, `produce` |
| `dispatch` | bus-internal routing of an event to its subscribers | `handle`, `process`, `route` |
| `tick` | advance one schedule step (engine-internal) | `step` (CLI-only), `update` |
| `step` | CLI command verb only — "advance N ticks" | — |
| `run` | execute a `System` or `Schedule` once | `update`, `execute` |
| `load` / `save` | (de)serialize manifest to / from disk | `read` / `write` (reserved for raw bytes) |
| `inspect` / `snapshot` | produce a JSONL dump of current world state | `dump`, `print`, `debug` |
| `new` | constructor; no parameters unless self-evident from type | `make`, `create`, `build` |

### Concept-keyed reservations

For concepts that cannot be derived from a Suffix + domain noun.

| Concept | Canonical term | Allowed forms | Do not use | Notes |
| --- | --- | --- | --- | --- |
| 2D vector (f32 pair) | `Vec2` | — | `Vector2`, `V2`, `Pair` | Lives in `core::math`. |
| World-space position | `Position` | — | `Pos`, `Location`, `Transform` | Newtype over `Vec2`; `Transform` reserved for v3 (rotation/scale). |
| World-space linear velocity | `Velocity` | — | `Vel`, `Speed`, `LinearVelocity` | Newtype over `Vec2`; `Speed` is the scalar magnitude. |
| Axis-aligned bounding box | `Aabb` | — | `AABB`, `BoundingBox`, `Box` | G.NAM.01 acronym-as-word. Field `half_extents: Vec2`. |
| Animation clip + metadata | `Clip` | `AnimationClip` (cross-module docs) | `Anim`, `Track`, `Sequence` | Future fields extend `Clip`, not `Animator`. |
| Animation playback state | `Animator` | — | `AnimationPlayer`, `AnimController` | Field `clip: Clip`, not `current`. |
| Generational entity handle | `Entity` | — | `EntityHandle`, `EntityRef` | `{ index: u32, generation: u32 }`. |
| Game world | `World` | — | `Universe`, `Scene`, `Registry` | Owns ECS storage and `TagRegistry`. |
| Component marker trait | `Component` | — | `Data`, `Datum` | `'static + Send + Sync`. |
| Tick schedule | `Schedule` | — | `Loop`, `Pipeline` | Ordered systems for one tick. |
| Event bus channel enum | `Channel` | `Channel::{ Input, Command, Domain, Marker, Presentation, CommandAck, Snapshot }` | strings at the type level | JSONL serializes variants as `lower-kebab-case`. |
| World-scoped singleton trait | `Resource` | — | `Singleton`, `Global`, `Config`, `Asset` | `'static + Send + Sync`; backs `TagRegistry`, `Gravity`, future per-game globals. `Asset` is reserved for v2+ disk-loaded data (textures, audio, animation), do not conflate. `Config` is misleading because resources mutate at runtime (`Time`, `Score`, `Rng`). |
| Event-driven if-then unit of game logic | `Rule` | — | `Handler`, `Trigger`, `Reaction`, `Listener`, `Script`, `Behavior` | One file in `rules/*.toml`; `event + match + do` triple. |

## Adding Table Entries

When adding a new entry:

1. Use a stable concept name, not a one-off implementation detail.
2. Choose one canonical term.
3. List allowed casing or spelling variants only when different contexts require
   them.
4. List known disallowed aliases when confusion is likely.
5. Add a short note explaining the scope or reason if the choice is not obvious.

## Conflict Handling

- If existing code uses a different term, follow the table for new code and only
  rename existing code when the task requires a rename.
- If two table entries appear to cover the same concept, stop and reconcile the
  table before adding more names.
- If a dependency, protocol, or external API requires a different spelling,
  document that spelling in `Allowed forms` instead of using it silently.
