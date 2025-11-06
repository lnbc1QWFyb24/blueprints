# Modular Prompt System

This directory contains reusable, composable prompt modules used to build complete role prompts for the Blueprints CLI at runtime.

Concepts
- Modules: Markdown snippets under `src/prompts/modules/` that encode specific, reusable guidance (rules, inputs, checks, procedures, etc.). Each module lives in its own file and is referenced by its filename slug without the `.md` extension. Many small modules have been consolidated into larger role/reference modules for simplicity.
  

Composition is defined in code via role profiles and module lists, keeping prompts simple and declarative in a single place.

Composing prompts
- CLI:
  - `cargo run -- compose review <module>`
  - `cargo run -- compose design <module> --out /tmp/design.md`
  - `cargo run -- compose implement-reviewer <module> --apply-tokens`
- Handlers: compose from built-in role profiles; no manifests are used.

Available modules (slugs)
- Core references
  - blueprints-reference
  - implementation-standards
  - parsing-rules
  - delivery-plan
  - interaction-style
  - tokens-output-protocol
  - workspace-constraints

- Roles
  - design
  - update
  - review
  - implement-builder
  - implement-reviewer

Tokens and inlining
- The system recognizes and may substitute these placeholders at runtime:
  - `${COMPLETED_TOKEN}`, `${CONTINUE_TOKEN}`, `${ERROR_TOKEN}`
  - `${BLUEPRINTS_DIR}` (workspace-relative path)
- The compose command can apply tokens (`--apply-tokens`) and inline `BLUEPRINTS.md` by default (disable with `--no-inline-blueprints`).

Tips
- Prefer modules for any content shared across roles; keep role-specific sections in dedicated role modules for clarity.
- Update manifests to reorder or swap modules without touching code.
- When adding a new reusable section, create a module and reference its slug in the appropriate manifest(s).
