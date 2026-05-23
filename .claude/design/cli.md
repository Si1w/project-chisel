# CLI

`clap`'s derive macros. Subcommands fall into two groups: **authoring**
(edit TOML files on disk) and **runtime** (drive the engine and read its
JSONL stream).

The CLI is the only authoring interface. There is no REPL, no TUI, no
scripting language. The CLI is a thin layer over the runtime crate;
every authoring subcommand resolves to a TOML mutation, every runtime
subcommand to a control-plane command event.

## Subcommand list (v0)

```text
# Authoring — mutate TOML files

chisel new <dir>
chisel add component <name> [--field NAME:TYPE]...      # v1+
chisel add entity    <name> [--component NAME]...
chisel add scene     <name>
chisel add rule      <id> --event EVENT [--match ...] [--do ACTION]...
chisel add input     <id> --key KEY --emit EVENT

chisel rule import   <file.jsonl>
chisel rule export   [--format jsonl|toml]

# Runtime — emit command events into the engine

chisel run           [root] [--dt 0.016] [--max-ticks N]
chisel step          [N]
chisel inspect       [--query QUERY]
chisel emit          <input-event-json> [root]   # routed through input.toml
```

With `--max-ticks`, `run` is batch mode: it advances the engine and exits.
Without `--max-ticks`, `run` is persistent JSONL session mode: it reads
newline-delimited `input` / `command` records from stdin and writes
newline-delimited output records to stdout until stdin reaches EOF.
Current minimal `emit` parses one `InputEvent` JSON object, applies
`input.toml`, queues the resulting domain events, and drains the rule
processor once. Session mode exposes the same path through
`command:simulate_input`.

Project codename (`chisel`) is provisional; the actual binary name is
decided before v0 ships.

## Authoring vs runtime contract

| Subcommand | Touches disk? | Emits a command event? |
| --- | --- | --- |
| `new` | Creates project directory | No |
| `add *` | Edits a TOML file | No |
| `rule import / export` | Reads/writes JSONL or TOML | No |
| `run` | Loads TOML | Spawns engine; loops `command:step` internally |
| `step` | No | Yes (`command:step`) |
| `inspect` | No | Yes (`command:inspect`); reads `snapshot` channel back |
| `emit` | No | Yes — translated into a `simulate_input` command that walks the input → mapper → domain pipeline (current minimal CLI executes that path directly) |

## Example session

Authoring:

```bash
chisel new bouncy
cd bouncy
chisel add entity Ball   --component Position --component Velocity --component Aabb --component Ball
chisel add entity Wall   --component Position --component Aabb     --component Wall
chisel add scene  main
chisel add rule   ball-bounce \
    --event collision \
    --match a.with=Ball \
    --match b.with=Wall \
    --do "reverse_velocity entity=a axis=from_normal" \
    --do "emit event=bounced payload.who=a"
```

Runtime:

```bash
chisel run . --max-ticks 600 > session.jsonl
printf '%s\n' \
  '{"channel":"command","type":"step","count":1}' \
  '{"channel":"command","type":"inspect","query":null}' \
  | chisel run .
```

Step-driven (agent in the loop):

```bash
chisel step 1   # advance one tick; read JSONL from stdout
chisel inspect  # dump world snapshot
chisel emit '{"type":"key_press","key":"Space"}'
```

## CLI error contract

- Authoring failures (invalid TOML, schema violation) exit non-zero with
  diagnostics on stderr; no partial writes.
- Runtime failures during `run` are emitted on the `command-ack` channel
  rather than crashing the process, except for unrecoverable IO errors.
- Malformed JSONL lines on stdin during `run` produce a `command-ack`
  with `status: "error"` and parsing continues with the next line.
