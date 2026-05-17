# Change Scope Rules

Read this before editing existing code, especially when a nearby cleanup or
style change looks tempting.

## Scope Discipline

- Every changed line should trace directly to the user's request.
- Touch only the modules needed to complete and verify the task.
- Match the surrounding style, naming, formatting, and error-handling pattern.
- Keep unrelated refactors, formatting sweeps, dependency changes, and renames
  out of the diff unless the user asked for them.

## Cleanup Boundaries

- Remove imports, variables, functions, comments, tests, and files that your own
  changes made obsolete.
- Do not delete pre-existing dead code unless the task is to remove it.
- If unrelated dead code, confusing comments, or risky patterns are discovered,
  mention them in the final response instead of editing them.
- If generated output or formatting tools touch unrelated files, inspect the
  diff and revert only your own accidental changes.

## Existing Worktree

- Assume unrelated uncommitted changes belong to the user.
- Do not overwrite, reset, or revert user changes unless explicitly asked.
- If user changes overlap with the requested edit, work with them and preserve
  their intent.
- Ask before taking destructive git actions.

