# Naming Conventions

Read this before introducing or changing project names, domain terms, public
identifiers, file names, config keys, CLI names, error names, or user-facing
labels.

These rules summarize local naming decisions for this repository. They
complement `.claude/rules/code-style.md`; when in doubt, follow the existing
module shape and keep the naming table current.

## Feature Work Requirement

- Before designing or implementing a feature, review this file.
- During design, identify every new project concept, module boundary, public
  type, public function, file name, config key, CLI name, error name, and
  user-facing label the feature introduces.
- If a concept is missing from the naming table, add it before using the name in
  code, docs, tests, configuration, or UI text.
- If a concept is already in the table, use the canonical term and allowed forms
  exactly as documented.

## Rust Names

- Use Rust-standard casing: modules, functions, methods, and locals in
  `snake_case`; types, traits, and enum variants in `UpperCamelCase`; constants
  and statics in `SCREAMING_SNAKE_CASE`.
- Treat acronyms as words in new `UpperCamelCase` names: use `Uuid`, not `UUID`.
- Prefer precise names over short vague names. Small local variables can be
  short; public types and cross-module APIs should say what they represent.
- Apply Rust conversion naming rules from `.claude/rules/code-style.md` after
  choosing the canonical term from this file.

## Module Boundaries

- Top-level modules should be bounded-context nouns.
- Inside a bounded module, avoid repeating the module name when the path already
  carries the context. Prefer `domain::Store` over `DomainStore` unless the type
  is commonly used outside that module and needs context.
- Use re-export aliases sparingly when they improve caller clarity without
  changing the underlying module ownership.

## Naming Table Rules

- Use the canonical term from the naming table whenever the concept already
  exists.
- If a concept is not in the table, add it to the table before using the new
  name in code, docs, tests, configuration, or UI text.
- Do not introduce synonyms, abbreviations, or alternate spellings unless the
  table explicitly allows them.
- Keep names self-documenting. Prefer precise names over short vague names.

## Naming Table

This template repository intentionally leaves the table empty. When starting a
real project from the template, add project-specific entries here before using
those names in code, docs, tests, configuration, or UI text.

| Concept | Canonical term | Allowed forms | Do not use | Notes |
| --- | --- | --- | --- | --- |

## Adding Table Entries

When adding a new entry:

1. Use a stable concept name, not a one-off implementation detail.
2. Choose one canonical term.
3. List allowed casing or spelling variants only when different contexts require
   them.
4. List known disallowed aliases when confusion is likely.
5. Add a short note explaining the scope or reason if the choice is not obvious.

## Conflict Handling

- If existing code uses a different term, follow the table for new code and only
  rename existing code when the task requires a rename.
- If two table entries appear to cover the same concept, stop and reconcile the
  table before adding more names.
- If a dependency, protocol, or external API requires a different spelling,
  document that spelling in `Allowed forms` instead of using it silently.
