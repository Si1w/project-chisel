# Simplicity Rules

Read this before adding abstractions, extension points, configuration, new
modules, generic helpers, or optional behavior.

## Default Bias

- Prefer the smallest solution that satisfies the explicit requirement.
- Keep happy-path behavior working before expanding edge-case handling.
- Use existing project patterns and helper APIs before creating new ones.
- Add structure only when it removes real complexity or matches an established
  local pattern.

## Avoid Speculative Work

- Do not add features that were not requested.
- Do not add configuration, flags, hooks, or strategy layers for hypothetical
  future needs.
- Do not create an abstraction for a single call site unless it clarifies a
  genuinely complex operation.
- Do not design for impossible states or unreachable error cases.

## Complexity Check

Before finishing a non-trivial change, ask:

- Can this be expressed with fewer types, functions, or branches?
- Did any abstraction appear before there were at least two real use cases?
- Would a maintainer understand the data flow without reading unrelated files?
- Is every new public API necessary for the current request?

If the answer points to avoidable complexity, simplify before closing out.

