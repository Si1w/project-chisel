# Rules

Rules are the **logic layer**: declarative bindings of `event → match →
action sequence`. They live in TOML, one file per rule, loaded at startup
and on `reload`.

## Event taxonomy (recap)

Five categories of events / commands live on the bus. Rules only see two
of them.

| Category | Source | Visible to rules? |
| --- | --- | --- |
| Command | agent / CLI (control plane) | No |
| Input | input source → input mapper | No (mapper converts these to domain) |
| Domain | ECS systems + rule emissions | **Yes** |
| Marker | ECS animation system | **Yes** |
| Presentation | rule emissions | No (output-only) |

**Why rules cannot subscribe to commands**: command events are the
agent's control-plane voice (`step`, `inspect`, etc.) and have a single
internal consumer — the command handler. Letting rules subscribe creates
two failure modes:

1. Emit-loops: a rule subscribing to `command:emit` and reacting by
   emitting another command produces an infinite cascade.
2. Boundary collapse: the agent observer can no longer distinguish "what
   the agent said" from "what a rule said the agent said".

The translation step from command → domain is intentional; it strips the
input origin so rules only care about what happened in the world, not
who asked for it.

## Rule file format (TOML)

One file = one rule. Example:

```toml
# rules/ball-bounce.toml

event = "collision"

[match.a]
with = ["Ball"]

[match.b]
with = ["Wall"]

[[do]]
action = "reverse_velocity"
entity = "a"
axis   = "from_normal"

[[do]]
action = "emit"
event  = "bounced"
[do.payload]
who = "a"
```

Layout:

- `event = "<type>"` — the event type to subscribe to. Must be a
  `domain` or `marker` event type.
- `[match.<param>]` — one section per event parameter. `collision` has
  two parameters (`a`, `b`); `tick` has none; custom events name their
  own. Inside, ECS query filters: `with = [...]`, `without = [...]`
  (v1+ adds `.where`).
- `[[do]]` — action array, executed in order. Each entry has an
  `action` field plus action-specific fields.

## v0 built-in events

- `tick` — no parameters; fires once per tick.
- `collision` — `a`, `b` are serialized `Entity` objects; `normal` is a
  `Vec2` object (`{ x, y }`).

## v0 built-in actions

| Action | Fields | Effect |
| --- | --- | --- |
| `set_velocity` | `entity`, `x`, `y` | Set `Velocity` of `entity`. |
| `reverse_velocity` | `entity`, `axis = "x" / "y" / "both" / "from_normal"` | Negate one or both velocity components. |
| `spawn` | `template`, `position` | Spawn an entity from a template. |
| `despawn` | `entity` | Remove `entity`. |
| `emit` | `event`, `payload` | Publish a `domain` event. |
| `play_animation` | `entity`, `name`, `priority` | Publish a `presentation` command and set `Animator`. |

## Match filter semantics

`with = [...]` lists tag names that must be present on the matched entity.
`without = [...]` lists tag names that must be absent. Tag names are
interned into `TagId`s through the world-scoped `TagRegistry`; each entity
stores them in one `TagSet` component.

## Failure semantics

If any action in a rule's `do` list fails (target entity already
despawned, component missing, etc.):

1. The remaining actions in that rule are aborted.
2. A `rule_action_failed` event is published on the `domain` channel with
   `rule`, `action_index`, and `reason` fields.
3. Other rules subscribing to the same triggering event continue to run
   independently.

This keeps a bad rule from cascading silently and lets agents detect
authoring mistakes via the event stream.

## v0 limits

- No `where`-style component value filters; only `with` / `without` on
  tag components.
- No rule priorities; rules sharing an event run in lexicographic file
  order. v1 may add explicit priorities.
- No rule-internal state; rules are pure event-to-action mappings.
