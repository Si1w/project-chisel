# Task Framing Rules

Read this before non-trivial feature work, bug fixes, refactors, public API
changes, or any request with more than one plausible interpretation.

## Required Behavior

- Restate the concrete goal before implementation when the request is broad.
- List assumptions that affect design, data shape, compatibility, or tests.
- Present alternatives when the choice changes scope, complexity, risk, or user
  experience.
- Ask for clarification when the answer cannot be inferred safely from the
  repository or the user's request.
- Push back when the requested route is likely to be more complex than the goal
  requires.

## Design Checkpoint

For new features and non-trivial changes, discuss the design before writing
code. Include:

1. Proposed data structures and type definitions.
2. Proposed function, module, or trait interfaces.
3. Key trade-offs and alternatives considered.
4. Naming review: existing naming table entries used, plus any new entries that
   must be added before implementation.
5. Open questions and assumptions.

Do not implement until the user confirms the design. Small local edits, typo
fixes, obvious one-line changes, and mechanical refactors are exempt.

## Confusion Handling

- Do not hide uncertainty by making a silent choice.
- If repository conventions conflict, name the conflict and choose the narrower
  path only if the choice is low risk.
- If requirements conflict, stop and ask instead of coding around the conflict.
- If a task appears to require external facts that may have changed, verify them
  before relying on them.
