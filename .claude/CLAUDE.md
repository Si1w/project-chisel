## Development Workflow

These principles govern *how* work gets done in this project. They apply to every feature, refactor, and non-trivial bug fix.

Additional project rules live in `.claude/rules/`. Read the relevant files there
before making changes that touch their topic.

## Agent Operating Principles

These principles apply to every non-trivial coding task. They bias toward fewer
wrong assumptions, smaller diffs, simpler designs, and verifiable outcomes. For
obvious typo fixes and one-line local changes, use judgment.

### Think Before Coding

- State assumptions before implementation when the request is ambiguous.
- If multiple interpretations are plausible, present them instead of silently
  choosing one.
- Push back when a simpler or lower-risk approach would meet the goal.
- Stop when confused. Name the unclear point and ask for clarification.

### Simplicity First

- Build the smallest correct solution that satisfies the request.
- Do not add features, abstractions, configuration, or flexibility that was not
  asked for.
- Avoid single-use abstractions unless they clearly reduce real complexity.
- If a solution can be much shorter without losing clarity or behavior, simplify
  it before moving on.

### Surgical Changes

- Touch only the files and lines needed for the requested change.
- Do not refactor, reformat, rename, or rewrite adjacent code unless it is
  required for the task.
- Match existing local style even when another style would be preferable.
- Clean up imports, variables, functions, and comments made obsolete by your own
  changes. Mention unrelated dead code instead of deleting it.

### Goal-Driven Execution

- Convert broad instructions into verifiable success criteria.
- For bugs, reproduce the failing behavior locally before claiming a fix.
- For refactors, verify behavior before and after the change when feasible.
- For multi-step work, state the plan with a verification step for each major
  milestone, then loop until the criteria are met or a blocker is clear.

### Rule Map

- Read `.claude/rules/task-framing.md` before design discussion, ambiguous work,
  public API changes, and non-trivial implementation.
- Read `.claude/rules/simplicity.md` before adding abstractions, configuration,
  new modules, or optional behavior.
- Read `.claude/rules/change-scope.md` before editing existing code, especially
  when unrelated cleanup is tempting.
- Read `.claude/rules/verification.md` before bug fixes, refactors, validation
  work, and any task that needs evidence of correctness.
- Read `.claude/rules/naming.md` before writing any feature and before
  introducing or changing project names, domain terms, public identifiers, file
  names, config keys, CLI names, error names, or user-facing labels. If the
  concept is not in the naming table, add it before using it.
- Read `.claude/rules/code-style.md` before Rust code changes.

### Design First

- **Define data structures and types before logic.** Write the `struct`, `enum`, and trait signatures first. Forcing the data model out early forces the business logic to be thought through, and the interfaces fall out naturally.
- **Interface before implementation.** Nail down function signatures and module APIs, then fill in the bodies. Contract-first, TDD-adjacent.
- **Naming table before feature code.** Before writing feature code, read `.claude/rules/naming.md` and identify whether new concepts, modules, public APIs, files, config keys, CLI names, errors, or user-facing labels need naming table entries.

### Naming & Structure

- **Self-documenting names.** `get_user_by_email()` beats `get_user2()` by a hundred. Long is fine; vague is not.
- **Single responsibility per function.** One function, one job. If it exceeds ~30 lines, question whether it should split.
- **Plan directory layout early.** Decide feature-organized vs. layer-organized up front — moving files later is expensive.

### Development Rhythm

- **Small, frequent commits.** Commit each working increment with a clear message. Future you will `git bisect` and thank past you.
- **Commit message format.** Use `type(scope): one sentence summary`, followed
  by a blank line and bullet points listing the key features, fixes, or docs
  changes.

  ```text
  type(scope): one sentence summary

  - List the key feature, fix, or docs change.
  - List relevant validation or follow-up when useful.
  ```

- **Happy path first, then edges.** Get the main flow working before chasing every edge case or error branch.
- **Tag `TODO` / `FIXME` consciously.** Mark temporary solutions explicitly — don't let them silently become permanent.

### Validation & Testing

- **Ship a runnable demo early.** The sooner it actually runs, the sooner you catch a wrong direction.
- **Unit-test the core logic.** Not 100% coverage — but key algorithms and data transformations must have tests.
- **A bug isn't fixed until you can reproduce it locally.** No guessing. Reproduce, then fix.
- **Keep Clippy strict.** Follow the Rust validation commands in `.claude/rules/verification.md` before closing out Rust changes.

### Defensive Mindset

- **Distrust external input.** API params, user input, third-party responses — validate at the boundary.
- **Error handling is not an afterthought.** For IO and network calls, design the failure path at the same time as the happy path.
- **Extract magic numbers into constants.** `const MAX_RETRY: u32 = 3;` beats a bare `3` scattered through the code.

### Documentation & Comments

- **Comments explain *why*, not *what*.** The code says what it does; comments explain the reasoning behind non-obvious choices.
- **Document counter-intuitive business logic.** If a reader would reasonably ask "why this way?", leave the answer in a comment.

---

## Core Rule — Discuss Design Before Implementation

**Before writing any code for a new feature, you MUST stop and discuss the design with the user first.** Present:

1. The proposed data structures and type definitions.
2. The proposed function / module interfaces.
3. Key trade-offs and alternatives considered.
4. Naming table entries used or added for the feature.
5. Open questions or assumptions.

**Do not start implementation until the user has confirmed the design is sound and efficient.** Spend ten minutes thinking and talking before you code — it prevents hours of rework.

This rule applies to every new feature and every non-trivial change. Small, localized edits — typo fixes, one-line bug fixes, mechanical refactors — are exempt.

Approved design documents live in `.claude/design/`, split by topic so each
file is read on demand rather than as one large doc:

- `.claude/design/problem.md` — problem statement, toy version, growth path,
  existing landscape.
- `.claude/design/architecture.md` — three layers (ECS / EventBus /
  Presentation + Input), three invariants, crate layout, sync/async boundary.
- `.claude/design/artifacts.md` — TOML/JSONL split, manifest directory layout,
  JSONL channel taxonomy.
- `.claude/design/ecs.md` — self-built ECS plan, reference points, deferred
  features.
- `.claude/design/physics.md` — AABB physics v0, `PhysicsEngine` trait,
  future swap path.
- `.claude/design/rules.md` — rule TOML syntax, event taxonomy, failure
  semantics.
- `.claude/design/cli.md` — `clap` derive subcommand list, authoring vs
  runtime split.
- `.claude/design/future.md` — premises, open questions, naming reservations,
  next steps.

Read the relevant topic file before touching code it covers; add or update
topic files when designing a new feature.
