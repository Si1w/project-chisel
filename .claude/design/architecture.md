# Architecture

Three layers with strict boundaries. Each owns a different kind of work,
and the boundaries between them are enforced by module structure, not
just convention.

```text
                ┌─────────────────────────────────────┐
                │  Authoring Artifacts (TOML)         │
                │   game.toml  components/  entities/ │
                │   scenes/  rules/  input.toml       │
                └────────────────┬────────────────────┘
                                 │ load on startup / explicit reload
                                 ▼
┌────────────────────────────────────────────────────────────────┐
│  Logic Layer (rule processor, sync between ticks)              │
│  in:  drains domain + marker events from EventQueue            │
│  out: (a) ECS world mutations  (b) presentation commands       │
└─────┬───────────────────────────────────────────┬──────────────┘
      │ mutations                                 │ commands
      ▼                                           ▼
┌────────────────────────────────────────────────────────────────┐
│  ECS Layer (sync per tick)                                     │
│  World • Components • Systems                                  │
│  Systems communicate via World only — never via the Event Bus  │
│  Some systems push domain/marker events into EventQueue        │
└─────┬───────────────────────┬──────────────────────────────────┘
      │ domain events         │ marker events
      ▼                       ▼
┌────────────────────────────────────────────────────────────────┐
│  Event Bus (tokio channels)                                    │
│  Channels: input | command | domain | marker | presentation    │
│              | command-ack | snapshot                          │
│  All bus traffic mirrors to stdout as JSONL with `channel`     │
└────────────────────────────────────────────────────────────────┘
                                ▲
                                │ input events
┌────────────────────────────────────────────────────────────────┐
│  Input Layer                                                   │
│  Input Source (v0: stdin JSONL channel="input")                │
│       ↓                                                        │
│  Input Mapper (consumes input.toml)                            │
│       ↓ publishes domain events                                │
└────────────────────────────────────────────────────────────────┘
```

## Three invariants

These are enforced by crate boundaries and code review.

1. **ECS systems do not subscribe to the event bus.** They consume world
   data via queries. Any external input arrives as component state, not
   as a bus subscription.
2. **The logic layer does not directly invoke ECS systems.** It runs between
   ticks, drains the world's `EventQueue`, and produces exactly two kinds
   of output: component mutations applied to the world, and presentation
   commands published to the bus.
3. **Bus traffic is segregated by channel.** Command events are not
   visible to rule subscriptions; rules subscribe to `domain` and
   `marker` only. See [rules.md](rules.md) for why.

## Layer boundaries — what each layer owns

| Layer | Owns | Does not own |
| --- | --- | --- |
| Authoring (TOML) | Source-of-truth game definition | Runtime state |
| Logic (rules) | Cause → effect mapping for queued game events | Per-frame data, system scheduling |
| ECS | High-frequency data + mechanical work (integrate, sample, detect) | Game-rule decisions |
| Event Bus | Runtime IO / observer fanout on `domain` / `marker` / `presentation` / etc. | Intra-ECS communication |
| Input | Raw input → domain event translation | Game-rule decisions about input |

## Module layout

Single binary crate. Submodules split work along the layer boundaries
above. A Cargo workspace split is deferred until either a subsystem
gains a divergent dependency set or the engine ships as a library.

```text
src/
  main.rs       # CLI binary entry
  math/         # Vec2 and other geometric primitives
  ecs/          # World, Entity, Component trait, query, schedule, system
  event/        # Channel, Bus, payload types
  physics/      # AabbEngine + PhysicsEngine trait
  runtime/      # manifest loader, async main loop, JSONL serializer
  cli/          # clap subcommands
```

Module names are generic; the project codename is provisional and is not
part of module names.

## Sync / async boundary

- Inside ECS: synchronous. `world.tick()` runs the schedule end-to-end
  with no awaits, returning when one tick is complete.
- Outside ECS: async. The runtime task reads stdin, writes stdout, drives
  the bus, and calls `world.tick()` on each step.
- Reason: ECS work is CPU-bound and benefits from a tight sync loop; IO
  and event delivery benefit from async. Mixing them inside the tick
  would introduce nondeterminism.
