# Future, Premises, and Open Questions

What the v0 design assumes, what it leaves open, what it reserves names
for, and where it expects to go.

## Premises

1. **Agent is the author**, not the player or a runtime puppeteer. Its
   primary output is TOML files. Runtime presence is limited to the
   control plane (`step`, `inspect`, etc.) plus optional
   `simulate_input` for testing.
2. **The manifest on disk is the source of truth.** Runtime mutations
   either come from physics/rule systems during a session, or from
   reloading the manifest. There is no editor-internal state.
3. **Async at the IO/event boundary; sync inside ECS ticks.**
   `world.tick()` is synchronous; the surrounding loop is async.
4. **ECS is self-written**, archetype-based in the style of `hecs`,
   single-threaded in v0. Parallel scheduling, sparse-set storage, and
   dynamic component registration are deferred.
5. **Physics detects, does not resolve.** Response is a rule decision.
   A `PhysicsEngine` trait isolates the v0 implementation from v2's
   `rapier2d` swap.
6. **CLI is the only authoring interface**, built with `clap` derive
   macros. No REPL, no TUI, no scripting language in v0.
7. **Event categories never mix freely.** Commands flow agent → engine
   handler; inputs flow source → mapper → domain; domain and marker
   are the only categories rules subscribe to.

## Open questions

1. **Component schema in v0**: closed compile-time set (current plan)
   vs. TOML-declared schemas from day one. Plan: closed for v0; spec
   the dynamic path before locking the API so v1 does not break
   callers.
2. **Rule conflict resolution**: when two rules on the same event
   mutate the same component, declaration order applies. A v1
   "conflicting rules" lint is on the list. Confirm declaration order
   is acceptable for v0 instead of erroring out.
3. **`Animator` placeholder semantics**: in v0 it stores
   `{current, elapsed, speed, looping}` but `AnimationSystem` only
   ticks `elapsed`. Should the presentation channel emit a synthetic
   `animation_finished` event for non-looping clips after a configured
   duration, even though no animation data exists? Plan: yes — keeps
   the downstream contract stable so v2 does not change observable
   behavior.
4. **Stdin JSONL robustness**: malformed lines produce `command-ack`
   with `status: "error"` and parsing continues. Confirm this beats
   fail-fast for an agent-driven workflow.

## Naming

Suffix patterns (`*Id`, `*Registry`, `*Set`, `*Engine`, `*Tx`/`*Rx`, `*Event`,
`*Command`, `*Error`, `*Schema`), verb conventions, and concept reservations
(`Vec2`, `Aabb`, `Position`, `Velocity`, `Clip`, `Animator`, `Entity`, `World`,
`Component`, `Schedule`, `Channel`) live in [`.claude/rules/naming.md`](../../rules/naming.md).
Compose new names from those tables; only fall back to adding a Concept-keyed
reservation when a name cannot be derived from a Suffix + domain noun.

## Next steps

1. Draft type signatures for the ECS layer (`src/ecs/`), event bus
   (`src/event/`), and physics (`src/physics/`, `PhysicsEngine` and
   `AabbEngine`) as a no-impl skeleton with `todo!()` bodies; review
   signatures independently of behavior.
2. Define `serde`-derived schemas (`*Schema` types) for `game.toml`,
   `entities/*.toml`, `scenes/*.toml`, `rules/*.toml`, `input.toml`;
   round-trip an example project.
3. Add the simplest end-to-end JSONL fixture test: two AABB boxes, one
   rule, `run --max-ticks 60`, expected output stream.
4. Fill in `World`, `Schedule`, `AabbEngine`, the rule engine, the
   input mapper, and the CLI on top of the skeleton.
5. Once the skeleton runs, return here and check the open questions
   against actual usage before promoting v0 to "stable".
