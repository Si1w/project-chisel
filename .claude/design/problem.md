# Problem

LLM agents have no first-class workflow for authoring 2D games. Existing
engines (Unity / Godot / Bevy) assume a human at a GUI editor; their scene
files are binary or human-centric, their scripting requires a runtime, and
their feedback loop is rendered pixels — none of which an agent can consume
or produce naturally.

This project builds a 2D game engine whose **authoring surface is a CLI**
and whose **artifacts are plain-text files (TOML for the scene tree, JSONL
for runtime streams)**. Agents read, diff, and write the game definition
with no custom tooling, and they observe runtime behavior as a JSONL event
stream instead of pixels.

Rendering is out of scope for v0. The engine still has a presentation
command channel so that when a renderer is added later, no upstream
architecture changes are required.

## Toy Version

A weekend prototype that runs end-to-end without rendering.

Capabilities:

- `new <dir>` scaffolds a project containing `game.toml`, `components/`,
  `entities/`, `scenes/`, `rules/`, `input.toml`.
- Built-in components only: `Position`, `Velocity`, `Aabb`, `Tag`
  (zero-sized marker), `Animator` (state placeholder, no real animation
  playback yet).
- Built-in domain events: `Tick`, `Collision`.
- Built-in actions inside rules: `set_velocity`, `reverse_velocity`,
  `spawn`, `despawn`, `emit`.
- AABB-only physics: integrate velocity, detect overlaps, publish
  `Collision` events. No collision resolution; rules decide the response.
- `step [N]` advances `N` ticks; `run --max-ticks N --dt 0.016` advances in
  a batch; `inspect` dumps a JSONL world snapshot.
- Stdin accepts a single JSONL stream with `channel` discriminator
  (`input` / `command`); stdout emits JSONL on `domain` / `marker` /
  `presentation` / `command-ack` / `snapshot` channels.

The toy is useful on its own: an agent authors two AABB boxes plus one
`Collision`-triggered rule that reverses velocity, then drives `step`
commands and observes a deterministic bounce loop in the JSONL stream.
That is already a runnable game artifact, version-controlled and
reproducible.

## Growth Path

- **v0 (toy)**: above.
- **v1**: custom component schemas declared in TOML (requires component
  registry + dynamic reflection); broad-phase spatial index (uniform
  grid); scripted input track for replay; `pause` / `resume`; multi-scene
  loading.
- **v2**: pluggable physics backend (`rapier2d` adapter behind the
  existing trait); real animation playback with marker events emitted
  from `AnimationSystem`; manifest hot-reload while running.
- **v3**: pluggable rendering frontend (Bevy-based or otherwise)
  consuming the presentation channel without engine changes; networked
  input source; parallel ECS scheduler.
- **v4+**: optional scripting plugin (WASM component) for rule actions
  that outgrow the declarative form.

Trigger to move from each version to the next: the previous capability is
demonstrably stable, and a real game in the working directory requires
the next capability.

## Existing Landscape

- **Bevy** — Rust ECS engine with a strong scene file story
  (`bevy_scene`), but authoring still assumes runtime registration of
  types from Rust code; not agent-authorable end to end.
- **Godot** — has a CLI and a text-based scene format (`.tscn`), but
  logic lives in GDScript which presumes an editor and requires a
  runtime VM.
- **hecs** — minimal archetype-based ECS library, no engine on top.
- **godot-rust (gdext)** — FFI bridge, not directly relevant to ECS
  design but informative for module boundaries between a runtime core
  and external clients.

The differentiator here is the authoring layer, not the runtime. ECS +
event-driven logic + AABB physics is well-trodden; what is new is the
strict separation between *TOML scene-tree authoring* and *JSONL runtime
observation*, combined with the constraint that the only client is a
command-line agent.
