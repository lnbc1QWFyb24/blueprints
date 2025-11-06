# Blueprints

Blueprints describe everything about a crate except the code implementation. They are the source artifacts from which code is generated and validated. Each crate includes a `blueprints` directory with the files below.

## Lifecycle Index (`00-lifecycle.md`)

- Purpose: Machine-readable ledger of deprecations and removals across all blueprint files.
- File: `blueprints/00-lifecycle.md`
- Scope: Records-only; ASCII; one record per line; non-destructive.
- Record schema: `<ID> | STATUS:<active|deprecated|removed> | REASON:<short>[ | EFFECTIVE:<semver|date>][ | REPLACE_BY:<ID>]`
- IDs: `<ID> ∈ R-### | S-###(.n)? | TV-### | C-###`
- Example: `R-017 | STATUS:deprecated | REASON:Replaced by X | EFFECTIVE:v1.4`
- Policy: Append-only; never edit or remove history; each entry is atomic and self-contained; acts as a stable reference for tooling, audits, and traceability.

## Requirements (`01-requirements.md`)

- Purpose: Define WHAT and WHY. Human-authored only.
- Format: Records-only; one line per requirement; ASCII; no headers/sections/tabs.
- Line schema: `R-### - <One-sentence description of WHAT and WHY>` (must end with `.`, `!`, or `?`).
- IDs: `R-###`; zero-padded; unique; stable; sorted ascending; gaps allowed; never reuse.
- Content rules: No implementation details, invariants, statuses, metadata, or multi-sentence items.
- Role: Source of truth for intent; Specs and tests derive from here.

## Spec (`02-spec.md`)

- Purpose: Define HOW to satisfy Requirements in a definitive, machine-readable way.
- Format: Strict; records-only; ASCII; no headers/sections/tabs.
- Clause schema (one per behavior): `S-###[.n] | R:R-###[,R-###...] | DO:<imperative, testable statement>[ | IF:<apis/types>][ | ER:<errors>][ | LM:<limits>][ | OB:<observability>]`
- Validation (core): `^S-([0-9]{3}(\.[0-9]+)?) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| DO:.+`
- IDs: `S-###` or `S-###.n`; unique; append-only; never renumber; file order is authoritative.
- Traceability: Each `S-*` references ≥1 `R-*`; every `R-*` is covered by ≥1 `S-*` when complete.
- Writing: MVP-first; imperative `DO` statements; `IF/ER/LM/OB` only when essential to implement/test.
- Editing policy: Agents may append/update `S` lines; preserve existing IDs; make atomic edits.

## Contracts (`03-contracts.md`)

- Purpose: Human-first, implementation-facing contracts captured in concise Markdown sections. Provide stable references `C-###` for code (`/// @contract(C-###)`), delivery tasks, and reviews. Focus on exact Rust types and external API details required for the near-term build; do not enforce machine-parseable schemas.
- Scope: Only currently relevant and settled items. Prefer specificity over breadth; avoid hypothetical placeholders.
- Structure: Each contract entry is a dedicated heading:
  - `### C-### — <Short Title>`
  - Kind: `Type` | `External API` | `Integration Note`
- Templates:
  - Kind: Type
    - Rust code (final form) in a code block including visibility, derives, and any `serde` hints.
    - Notes listing invariants, error kinds, and display/serialization requirements.
  - Kind: External API
    - Service and base URL.
    - Endpoint: `<METHOD> <path>` with purpose.
    - Auth scheme; required headers.
    - Request/Response: JSON examples and/or Rust request/response types.
    - Errors: status codes/messages mapped to local error types.
    - Performance: timeouts, retries/backoff, pagination, rate limits, idempotency.
    - Docs: canonical links.
  - Kind: Integration Note
    - Guidance for retries, timeouts, telemetry, schema evolution, and versioning/migration.
- IDs: `C-###`; zero-padded; unique; append-only. Preserve existing IDs and ordering. Deprecate with `DEPRECATED:<reason>` instead of renumbering.
- Traceability: Items are referenceable from code/tests via `/// @contract(C-###)`. Contracts should enable satisfying relevant Spec clauses and be referenced in the Delivery Plan.
- Editing policy: Human-first Markdown; keep minimal and tidy. Prefer appending or narrow edits; include only settled guidance.

## Test Vectors (`04-test-vectors.md`)

- Purpose: Define canonical test cases that validate correctness.
- Audience: Engineers, QA, automation, regression harnesses.
- Nature: Definitive, executable, evolving with real-world findings.
- Content: Core vectors from requirements (functional and boundary), and regression vectors for every discovered bug. Structured for deterministic, automated verification with concrete inputs, expected outputs, invariants, and references to `R-###`, `S-###`, `S-###.n`.
- Traceability: Each test vector references spec clauses.
- Editor: Agent/LLM may freely update.
- Excludes: Implementation details of test harnesses.

## Delivery Plan (`05-delivery-plan.md`)

- Purpose: Define concrete steps to implement the Spec and Test Vectors.
- Audience: Engineers, contributors, reviewers.
- Nature: Actionable, checklist-driven.
- Content: Milestones (task groupings) and checklist items referencing Spec IDs (`S-###`, `S-###.n`), Test Vectors (`TV-###`), and Contracts (`C-###`).
- Completion: Complete only when all spec clauses and associated test vectors and contracts have corresponding checklist items.
- Editor: Agent/LLM may freely update to ensure full coverage.
- Excludes: Architecture or spec content.

## Dependency Chain

1. Requirements → Source of truth (WHAT/WHY).
2. Spec → Derived from Requirements (HOW); must comprehensively cover all Requirements.
3. Contracts → Human-first concrete types, external API surfaces, and integration notes (`C-*`).
4. Test Vectors → Derived from Spec and Contracts; validate correctness and regressions; provide confidence that passing tests reflect a mostly correct system.
5. Delivery Plan → Derived from Spec, Contracts, and Test Vectors; defines the implementation path.
6. Lifecycle → Cross-cutting index of entity status (active/deprecated/removed) across all files; append-only; never modifies source content.

Cross-references are mandatory at every level.

## Doc–Code Sync Policy

- Blueprints drive code. If a code change introduces or modifies behavior, types, or external APIs that are not yet captured in the blueprints, update the relevant blueprint files in the same PR to re-establish alignment.
- Update the appropriate files: `01-requirements.md`, `02-spec.md`, `03-contracts.md`, `04-test-vectors.md`, `05-delivery-plan.md`, and `00-lifecycle.md` when statuses change.
- Do not merge code-only changes that diverge from the documented blueprints.

## Rule of Thumb

| File          | Focus                               | Content Type                                                        | Excludes                                      |
| ------------- | ----------------------------------- | ------------------------------------------------------------------- | --------------------------------------------- |
| Requirements  | External promises (WHAT/WHY)        | Goals, scenarios, interfaces                                        | Implementation details                        |
| Spec          | Internal guarantees (HOW)           | Behavioral clauses, invariants, logic rules                         | Tasks, milestones                             |
| Contracts     | Concrete definitions (WHAT EXACTLY) | C-### sections: Rust types, external API details, integration notes | Business logic, algorithms, speculative items |
| Test Vectors  | Correctness validation              | Inputs, outputs, references                                         | Harness/framework details                     |
| Delivery Plan | Execution steps (IMPLEMENT)         | Milestones, checklists, cross-references                            | Architecture/spec content                     |
| Lifecycle     | Entity status (lifecycle)           | Append-only records of active/deprecated/removed                    | Behavior, architecture, source edits          |
