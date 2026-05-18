# Physics

v0 covers AABB integration and overlap detection. It does **not** resolve
collisions. The response is a rule's job.

A `PhysicsEngine` trait isolates the implementation so v2 can swap in
`rapier2d` (or another engine) without touching upstream layers.

## v0 capability

For every tick:

1. **Integrate**: `Position += Velocity * dt` for every entity with both
   components.
2. **Broad phase**: naive — every pair. Acceptable at toy scale; a
   uniform grid is the v1 upgrade.
3. **Narrow phase**: AABB-vs-AABB overlap. No SAT, no rotated boxes, no
   continuous collision.
4. **Emit**: on overlap, publish to the `domain` channel:

   ```json
   {"channel":"domain","type":"collision","a":<entity>,"b":<entity>,"normal":[nx,ny]}
   ```

5. **Resolve**: not done. Rules decide whether to reverse velocity,
   despawn, apply damage, etc. by subscribing to `collision`.

## Trait

```rust
pub trait PhysicsEngine {
    fn step(&mut self, world: &mut World, bus: &BusTx, dt: f32);
}

pub struct AabbEngine;

impl PhysicsEngine for AabbEngine {
    fn step(&mut self, world: &mut World, bus: &BusTx, dt: f32) {
        // 1. integrate
        // 2. detect (O(n^2) pair scan)
        // 3. publish Collision events
    }
}
```

Selecting the engine in `game.toml`:

```toml
[physics]
engine = "aabb"      # v0: only "aabb"

[physics.gravity]    # optional; defaults to zero (top-down games)
x = 0.0
y = -9.81
```

Canonical TOML form for any `Vec2` field is a sub-section
(`[parent.field]` with `x` / `y` lines). Inside array-of-tables
(`[[do]]`, `[[map]]`, `[[entities]]`), the equivalent is a subtable on
the latest array element — e.g., `[do.position]` for `Spawn` action,
`[do.payload]` for `Emit` action, `[entities.overrides.position]` for a
scene instance override. The `Vec2` Rust type accepts inline-table and
dotted-key forms too, but every example in this design and every
loader-generated file uses the sub-section form.

## v0 → v2 upgrade plan

| Step | What changes | What stays |
| --- | --- | --- |
| Add broad-phase grid | `AabbEngine` internals | trait, events, rule API |
| Swap in `rapier2d` | new `Rapier2dEngine` implementing `PhysicsEngine` | trait, events, rule API |
| Add collision resolution as opt-in resolve flag | trait signature gains a resolve mode | event schema |

The trait is the only public boundary that must stay stable across these
upgrades.

## What is intentionally absent in v0

- Rotation, friction, restitution.
- Constraint solver, joints.
- Continuous collision detection.
- Triggers vs. solids distinction (everything is a trigger — emit-only).

These are all v2-or-later. v0 keeps the model small enough that the rule
layer is the interesting surface for now.
