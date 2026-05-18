# ECS

Self-written, archetype-based, single-threaded in v0. Designed so v1+ can
add sparse-set storage, parallel scheduling, and dynamic component
registration without breaking the v0 API.

## Reference points

| Library | Read for | Skip |
| --- | --- | --- |
| `hecs` | Overall API style; archetype storage; query interface; single-file readability | No parallel scheduler |
| `bevy_ecs` | `World` / `Schedule` / `SystemParam` / `Query` filter DSL; change detection; resources | Tight coupling to `bevy_app` |
| `gdext` (godot-rust) | FFI boundaries, runtime ↔ external client separation | Not an ECS |

v0 starts as `hecs`-style storage with `bevy_ecs`-style query filter
naming.

## Public surface (sketch)

Not final; signatures land via the next implementation plan and pass the
naming table before code is written.

```rust
pub struct Entity { pub index: u32, pub generation: u32 }

pub trait Component: 'static + Send + Sync {}

pub trait Resource: 'static + Send + Sync {}

pub struct World { /* archetype storage, entity allocator, resources */ }

impl World {
    pub fn spawn(&mut self) -> EntityBuilder<'_>;
    pub fn despawn(&mut self, e: Entity) -> Result<()>;
    pub fn get<T: Component>(&self, e: Entity) -> Option<&T>;
    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<&mut T>;

    // Split query API: ReadOnlyQuery bound on the &self path rules out
    // &mut T and prevents unsound mutable aliasing through &World.
    pub fn query<Q: QueryFetch + ReadOnlyQuery>(&self) -> QueryBuilder<'_, Q>;
    pub fn query_mut<Q: QueryFetch>(&mut self) -> QueryBuilder<'_, Q>;

    pub fn resource<R: Resource>(&self) -> Option<&R>;
    pub fn resource_mut<R: Resource>(&mut self) -> Option<&mut R>;
}

// ECS systems take &mut World only — invariant 1 keeps `&Bus` out of
// `run`. Systems emit via `world.resource_mut::<EventQueue>()`.
pub trait System: Send {
    fn name(&self) -> &str;
    fn run(&mut self, world: &mut World, ctx: &TickContext);
}

pub struct Schedule { /* ordered systems for one tick */ }

impl Schedule {
    pub fn tick(&mut self, world: &mut World, ctx: TickContext);
}
```

## v0 built-in components

- `Position(Vec2)` — newtype with `Deref<Target = Vec2>`.
- `Velocity(Vec2)` — same.
- `Aabb { half_extents: Vec2 }`.
- `TagSet` — **single bitset component** (`u128` inline) holding
  interned `TagId`s. One `TagSet` per entity. See note below.
- `Animator { clip: Clip, elapsed: f32, speed: f32, looping: bool }` —
  placeholder; `AnimationStep` only advances `elapsed` in v0.

**Tag note**: tags are **dynamically named**, not Rust types. The
world-scoped `TagRegistry` resource interns each tag name (`"Player"`,
`"Wall"`, ...) to a `TagId(u16)`; v0 caps at 128 distinct names because
`TagSet` is a `u128` inline bitset (zero allocation, O(1) `contains` and
bulk set ops). Query filters work on `TagId`, not Rust types. v1 widens
the bitset if a real game saturates 128 tags.

This differs from the hecs / bevy convention of "one Rust struct per
tag" because our agent authors the tag set in TOML at design time and
cannot extend a Rust enum. The dynamic registry is the trade-off — we
lose compile-time tag typing but gain TOML-friendly authoring.

## v0 systems

Scheduled in this order each tick (ECS systems only):

1. `PhysicsIntegrate` + `CollisionDetect` — bundled inside `AabbEngine`
   (which `impl System`); publishes `Collision` events into the
   `EventQueue` resource.
2. `AnimationStep` — advance `Animator.elapsed`; in v0 no marker events
   fire.

## Deferred (do not block v0 API decisions on these)

- Sparse-set storage for hot mutation paths.
- Parallel system execution via topological scheduling.
- Dynamic component registration / TOML-declared component schemas.
- Change detection / observer queries.
- Bevy-style typed `Events<T>` resources (we use a single `EventQueue`
  with `domain` and `marker` FIFOs; one consumer = `RuleProcessor`).

The v0 API should not foreclose these. Specifically: do not expose
internal archetype indices in the public type; keep `Query` parameters
opaque enough to swap storage; let `Schedule` be a `Vec<Box<dyn System>>`
for now so swapping it for a parallel scheduler is a single-crate change.
