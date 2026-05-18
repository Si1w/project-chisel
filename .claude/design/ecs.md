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
pub struct Entity(u32, u32 /* generation */);

pub trait Component: 'static + Send + Sync {}

pub struct World { /* archetype storage, entity allocator */ }

impl World {
    pub fn spawn(&mut self) -> EntityBuilder<'_>;
    pub fn despawn(&mut self, e: Entity) -> Result<()>;
    pub fn get<T: Component>(&self, e: Entity) -> Option<&T>;
    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<&mut T>;
    pub fn query<Q: Query>(&self) -> QueryIter<'_, Q>;
}

pub trait System {
    fn run(&mut self, world: &mut World, bus: &BusTx);
}

pub struct Schedule { /* ordered systems for one tick */ }

impl Schedule {
    pub fn tick(&mut self, world: &mut World, bus: &BusTx);
}
```

## v0 built-in components

- `Position { x: f32, y: f32 }`
- `Velocity { x: f32, y: f32 }`
- `Aabb { half_w: f32, half_h: f32 }`
- `Tag` — zero-sized marker; one Rust type per named tag (`Player`,
  `Wall`, `Ball`, ...). See note below.
- `Animator { current: String, elapsed: f32, speed: f32, looping: bool }`
  — placeholder; `AnimationSystem` only advances `elapsed` in v0.

**Tag note**: in Rust, each named tag (`Player`, `Wall`, ...) is its own
zero-sized type implementing `Component`. The manifest references them
by string; v0 maps strings to types via a closed enum/registry. v1's
dynamic component path replaces this string → type mapping with
reflection.

## v0 systems

Scheduled in this order each tick:

1. `PhysicsIntegrate` — apply velocity to position.
2. `CollisionDetect` — naive O(n²) AABB pair check; publish `Collision`
   events.
3. `RuleApply` — consumes events queued during the tick, applies their
   mutations; this is the bridge between bus and world.
4. `AnimationStep` — advance `Animator.elapsed`; in v0 no marker events
   fire.

## Deferred (do not block v0 API decisions on these)

- Sparse-set storage for hot mutation paths.
- Parallel system execution via topological scheduling.
- Dynamic component registration / TOML-declared component schemas.
- Change detection / observer queries.
- Resources (singletons) — until a system actually needs one.

The v0 API should not foreclose these. Specifically: do not expose
internal archetype indices in the public type; keep `Query` parameters
opaque enough to swap storage; let `Schedule` be a `Vec<Box<dyn System>>`
for now so swapping it for a parallel scheduler is a single-crate change.
