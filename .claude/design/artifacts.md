# Artifacts

Two formats, two roles:

- **TOML** is the source of truth for the scene tree — the static,
  version-controlled definition of the game. Human- and agent-readable;
  supports comments and sections; loaded once at startup or on explicit
  reload.
- **JSONL** is the runtime stream — events, commands, and snapshots
  flowing in and out of the engine. Line-oriented; agent-friendly for
  incremental parsing; ephemeral.

> Mnemonic: **TOML = what the game looks like. JSONL = what is happening.**

## Manifest directory layout

```text
my-game/
  game.toml          # top-level: world params, tick rate, module declarations, schema version
  components/*.toml  # component schemas (v1+; v0 uses built-in components only)
  entities/*.toml    # entity templates / archetypes
  scenes/*.toml      # entity instances + initial events
  rules/*.toml       # event → match → action bindings
  input.toml         # input mapping (raw input → domain event)
```

In v0:

- `game.toml` is required.
- `entities/` and `scenes/` are required.
- `rules/` and `input.toml` are optional (a scene with no rules and no
  inputs is a valid, if static, game).
- `components/` is reserved for v1; v0 ignores it.

## Content-to-artifact map

| Content | Location | Format |
| --- | --- | --- |
| Top-level world config (tick rate, schema version) | `game.toml` | TOML |
| Component schemas (v1+) | `components/*.toml` | TOML |
| Entity templates | `entities/*.toml` | TOML |
| Scenes (instances + initial events) | `scenes/*.toml` | TOML |
| Rules (event → match → action) | `rules/*.toml` | TOML |
| Input mappings | `input.toml` | TOML |
| Compiler diagnostics | `diagnostic` channel | JSONL |
| Player input (real or simulated) | input channel | JSONL |
| Game-meaningful events | domain channel | JSONL |
| Animation marker events | marker channel | JSONL |
| Presentation commands (animation / SFX cues) | presentation channel | JSONL |
| Control-plane commands (step / inspect / save) | command channel | JSONL |
| Engine acknowledgements | command-ack channel | JSONL |
| Inspect / state dump | snapshot channel | JSONL |

## JSONL channel discriminator

Every JSONL line in either direction carries a `channel` field. Example
flow for a player jump:

```jsonl
{"channel":"input","type":"key_press","key":"Space"}
{"channel":"domain","type":"player_jumped","actor":{"index":1,"generation":0}}
{"channel":"presentation","type":"play_animation","entity":{"index":1,"generation":0},"clip":"jump","priority":5}
```

The Rust payload types do not store `channel`; the runtime wraps every
payload in a `BusEnvelope` before JSONL serialization. v0 wire shapes are
object-shaped and match derived serde output:

- `Entity` serializes as `{ "index": u32, "generation": u32 }`.
- `Vec2` serializes as `{ "x": f32, "y": f32 }`.
- `DomainEvent.payload` is an object flattened into the envelope; loaders
  reject custom domain payloads that are not TOML/JSON objects.
- `input.toml` maps raw `InputEvent` objects to domain events. Match
  sections bind entities by tag filters, and `"$name"` strings in
  `emit.payload` substitute the matched entity.

The eight channels:

| Channel | Direction | Who writes | Who reads |
| --- | --- | --- | --- |
| `diagnostic` | outbound | compiler commands | agent / human |
| `input` | inbound | input source (stdin / scripted file) | input mapper |
| `command` | inbound | agent / CLI | command handler (single internal consumer) |
| `domain` | outbound + internal | ECS systems, rules | rules, agent observer |
| `marker` | outbound + internal | ECS animation system | rules, agent observer |
| `presentation` | outbound | rules | renderer (future), agent observer |
| `command-ack` | outbound | command handler | agent |
| `snapshot` | outbound | runtime (on `inspect`) | agent |

In v0 all channels mux through stdin/stdout. Splitting to separate file
descriptors is deferred to v1.

The `diagnostic` channel is not a runtime bus subscription in v0. It is
the compiler/tool output channel used by commands such as `compile`.

In persistent session mode, stdin is the append-only command/input source
and stdout is the append-only observation stream. The engine writes
domain, marker, presentation, command-ack, and snapshot records; clients
read those records and decide what to append next.

## Replay model

Recording the JSONL stream lets the engine replay a session
deterministically:

- Capture `input` and `command` lines coming in.
- Capture `domain`, `marker`, `presentation`, `command-ack`, `snapshot`
  lines going out (as ground truth for diffing).
- Replay = feed the captured input/command back into a fresh engine
  session with the same TOML manifest, then diff outputs.

Replay is implicit in the v0 design — no separate code path needed.
