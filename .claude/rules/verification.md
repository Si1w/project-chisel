# Verification Rules

Read this before bug fixes, refactors, validation work, behavior changes, and
any task where correctness must be demonstrated.

## Success Criteria

- Convert broad tasks into observable success criteria.
- Tie each major implementation step to a verification step.
- Prefer focused tests for core logic and data transformations.
- For user-facing or workflow changes, verify the runnable path, not only the
  individual function.

## Bugs

- Reproduce the failing behavior locally before implementing the fix.
- Add or identify a failing test, fixture, command, or minimal reproduction when
  feasible.
- After fixing, rerun the reproduction and the closest relevant test suite.
- Do not claim a bug is fixed without evidence.

## Refactors

- Establish the current behavior before changing structure when feasible.
- Keep public behavior stable unless the user requested a behavior change.
- Run the same relevant checks before and after the refactor when practical.
- Avoid widening the refactor beyond the stated goal.

## Rust Validation

Rust code changes must satisfy `.claude/rules/code-style.md`.

CI/CD-aligned checks:

- Every CI/CD job must run tests, except the macOS package job.
- The macOS package job may skip tests only when it packages artifacts that have
  already passed the normal test jobs.
- Local validation should mirror the non-package CI jobs:

```bash
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- \
    -W clippy::missing_errors_doc \
    -W clippy::missing_panics_doc \
    -W clippy::tabs_in_doc_comments
```

Before closing out Rust changes, also run the strict local check when feasible:

```bash
cargo clippy --all-targets --all-features -- -W clippy::pedantic -D warnings
```

If a command cannot be run, report why and state the residual risk.
