# Blueprints CLI

Blueprints CLI is a companion tool for Spec‑Driven Development / Agentic Engineering. You describe what the software must do in clear, human‑readable blueprints, and LLM agents generate the code that implements those requirements and specs. This shifts developer time toward defining the intent (the what and the why) while agents perform the repetitive implementation work.

## Why Rust + Codex CLI

This system targets Rust projects and is designed to be driven by the Codex CLI.

- Rust’s strict compiler and linting create strong boundaries that keep agent output on the rails.
- The workflow emphasizes iterative feedback loops so agents can correct mistakes quickly.
- The goal is test‑driven: extensive tests validate correctness so you don’t need to review every line of code.
- In practice, Codex CLI has proven reliable for this workflow and tends to do what it says it has done.

## An Experiment

This project is an experiment in raising the abstraction level of software development. Just as the industry moved from assembly to C with a compiler, we explore moving from today’s high‑level languages (Rust, C, etc.) to structured natural language “blueprints.” The aspiration is that code generation becomes “just another step” before compile and deploy, where the blueprints become the new source of truth and the code is regenerated for each release.

## Getting Started

1. Install Rust

- Install via rustup: see https://rustup.rs
- Verify: `rustc --version` and `cargo --version`

2. Install Codex CLI

- Follow the [Codex CLI installation guide](https://github.com/openai/codex) (ensure `codex --version` works).

3. Build this CLI

- From the repo root: `cargo build --release`
- Create an alias to the built binary: `alias blueprints="$(pwd)/target/release/blueprints"`
- Show help: `blueprints --help`
- Optional: persist the alias so it’s available in new shells:
  - zsh (macOS default): from repo root `echo "alias blueprints='$(pwd)/target/release/blueprints'" >> ~/.zshrc && source ~/.zshrc`
  - bash: from repo root `echo "alias blueprints='$(pwd)/target/release/blueprints'" >> ~/.bashrc && source ~/.bashrc`

## Blueprints Layout

The CLI resolves a `blueprints` directory relative to your workspace using the target you specify:

- `--module <path>` — search within that module directory first.
- `--crate <name>` — search within that crate directory (supports both `<workspace>/<name>` and `<workspace>/crates/<name>`).
- No flags — default to the current working directory.

If no existing `blueprints` directory is found, the CLI creates one in the best matching location so agents always have a workspace. See BLUEPRINTS.md for file semantics and naming conventions.

## Global Options

- `--summarize` — Enables live Codex output summarization in long‑running flows.
- `--sound <name>` — On macOS, play a system chime on success (where supported).
- `--list-sounds` — On macOS, list available chime names and exit (where supported).

Environment configuration (advanced):

- `MAX_BUILDER_ITERS` (default 50) — Iteration cap for builder loops.
- `MAX_REVIEWER_ITERS` (default 100) — Iteration cap for reviewer loops.
- `LOOP_SLEEP_SECS` (default 0.2) — Delay between iterations.

## Command Summary

Each command orchestrates Codex CLI with purpose‑built prompts. Use `--crate <package>` to target a crate by package name or `--module <path>` when you want to scope a nested module (e.g. `crates/crate_b/module_a`). Omit both to operate relative to the current directory.

### requirements

Use when defining or refining WHAT and WHY in `01-requirements.md`.

- Design: `blueprints requirements --crate crate_a --mode design`
  - Starts a design session for new requirements.
- Update: `blueprints requirements --module crates/crate_b/module_a --mode update`
  - Evolves existing requirements while preserving record structure.

Flags: (required) `--mode <design|update>`; optional targeting `--crate <name>`, `--module <path>`, sound options `--sound <name>`, `--list-sounds`

### specs

Use when drafting or updating the definitive HOW in `02-spec.md`.

- Design: `blueprints specs --crate crate_a --mode design`
- Update: `blueprints specs --module crates/crate_b/module_a --mode update`

Flags: (required) `--mode <design|update>`; optional targeting `--crate <name>`, `--module <path>`, sound options `--sound <name>`, `--list-sounds`

### contracts

Use when capturing concrete types, external API surfaces, or integration notes in `03-contracts.md`.

- Run: `blueprints contracts --crate crate_a`
  - Guides creation/refinement of `C-###` entries used by code and tests.

Flags: optional targeting `--crate <name>`, `--module <path>`, sound options `--sound <name>`, `--list-sounds`

### tests

Use when generating and curating canonical test vectors in `04-test-vectors.md` and adjacent tests.

- Run: `blueprints tests`
  - Reviewer/Builder loop to propose test coverage and implement tests iteratively.

Flags: `--sound <name>`, `--list-sounds`

### delivery

Use when shaping the implementation plan in `05-delivery-plan.md`.

- Run: `blueprints delivery --module crates/crate_b/module_a`
  - Reviewer/Builder loop to turn specs into an actionable checklist with cross‑references.

Flags: optional targeting `--crate <name>`, `--module <path>`, sound options `--sound <name>`, `--list-sounds`

### implement

Use when translating approved blueprints into code with CI feedback.

- Run: `blueprints implement --crate crate_a`
  - Reviewer/Builder loop translates the plan into code.
  - Runs host checks when a `Cargo.toml` exists: `cargo fmt --check`, `cargo clippy`, `cargo check`, and `cargo nextest run`.
  - If checks fail and Codex CLI is available, a CI‑fixer loop proposes and applies fixes until CI is clean or limits are reached.

Flags: optional targeting `--crate <name>`, `--module <path>`, sound options `--sound <name>`, `--list-sounds`

## Tips

- Verify Codex CLI is on PATH: `codex --version`
- Use `--summarize` for see summarize agent logs rather than full firehose.
- On macOS, `--list-sounds` shows valid names for `--sound`.

## Troubleshooting

- Codex CLI missing: ensure `codex` is installed and on PATH.
- Cargo missing: install Rust toolchain and verify `cargo --version`.
- Blueprints not found: re-run with `--crate <name>` or `--module <path>` to point at the right target; the CLI will create `blueprints/` automatically if it’s missing.

## Further Reading

- See `BLUEPRINTS.md` for the blueprint file formats, lifecycle rules, and cross‑reference policy.
