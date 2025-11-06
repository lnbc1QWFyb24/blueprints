# Implement Delivery Plan and Incorporate Reviewer Feedback

OBJECTIVE

- Implement the Delivery Plan end-to-end while incorporating the latest Reviewer feedback.
- Make focused, small, verifiable changes (scoped chunks) per iteration.

INPUTS

- All paths are relative to the current working directory (CWD). Do not `cd`; do not read or write outside CWD.
- Spec (authoritative; read-only): ${BLUEPRINTS_DIR}/02-spec.md
- Contracts (read-only): ${BLUEPRINTS_DIR}/03-contracts.md (handwritten Markdown numbered list)
- Test Vectors (read-only): ${BLUEPRINTS_DIR}/04-test-vectors.md (records-only TV lines)
- Delivery Plan (checkbox updates only; do not edit text): ${BLUEPRINTS_DIR}/05-delivery-plan.md
- Requirements (read-only): ${BLUEPRINTS_DIR}/01-requirements.md
- Reviewer Feedback (numbered list) and/or Remaining Work List (from a prior builder iteration).

PARSING (fast, exact)

- Requirements: lines matching `^R-[0-9]{3} - (.+)[.!?]$`; extract R-id and text; preserve numeric order.
- Spec: records-only S-lines matching `^S-([0-9]{3}(\\.[0-9]+)?) \\| R:(R-[0-9]{3}(,R-[0-9]{3})*) \\| DO:(.+?) \\| TITLE:([^|]+)( \\| .+)?$` (optional IF/ER/LM/OB may follow);
  extract S-id, R-id list, DO text, and required TITLE. TITLE must appear immediately after DO: and before any optional IF/ER/LM/OB fields.
  Use `TITLE` verbatim for concise, stable milestone titles.
- Contracts: Markdown with `C-###` items; skim for Rust type names/code blocks; external endpoints (METHOD PATH), auth schemes, request/response shapes, error mapping, and doc links.
- Test Vectors: records-only TV lines matching `^TV-([0-9]{3}) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| S:(S-[0-9]{3}(\\.[0-9]+)?(,S-[0-9]{3}(\\.[0-9]+)?)*) \| L:([UIP](,[UIP])*) \| GIVEN:.+ \| WHEN:.+ \| THEN:.+`;
  extract TV id, S-id list, R-id list, and Level set.

DELIVERABLES

- Code matching Spec APIs/invariants and error contracts, plus concrete Contracts (types and external integrations) defined in 03-contracts.md where applicable.
- Tests that implement relevant TV-<nnn> for the targeted S-ids; include `/// @covers(R-...)` and `/// @tv(TV-...)` on each test; include TV id in the test name when feasible.
- Docs: Maintain the module's documentation under its docs/ directory (within the target module). Provide usage examples and basic information; detailed behavior belongs in code doc comments. Do not edit Delivery Plan text.
- Delivery Plan: check off completed tasks only; do not rename/remove. If a Delivery Plan item is unchecked but appears implemented in the codebase, verify it against Spec/Contracts/Test Vectors and, if correct, check it off; if not fully correct, fix the implementation and then check it off.

TRACEABILITY TAGGING

- Add Rust doc-comment tags to link code/tests to Requirements, Spec, Contracts, and Test Vectors so `sg --lang rust -p '<pattern>'` can locate them structurally.
  - Implementations (functions, methods, types, trait impls): `/// @impl(R-...[,R-...])`; optionally `/// @s(S-...[,S-...])`; and `/// @contract(C-###)` referencing the relevant Contracts item when applicable.
  - Tests (`#[test]` functions): `/// @covers(R-...[,R-...])` and `/// @tv(TV-...)`.
  - Optional module roll-up: `//! @s(S-...)` on module/crate roots when helpful.
- Example `sg` queries:
  - Implementations: `sg --lang rust -p '#[doc = "@impl(R-017)"]'`
  - Tests covering a requirement: `sg --lang rust -p '#[test] #[doc = "@covers(R-017)"]'`
  - Tests for a vector: `sg --lang rust -p '#[doc = "@tv(TV-015)"]'`
  - Contract touch points: `sg --lang rust -p '#[doc = "@contract(C-001)"]'`
- Tag coverage requirement: apply tags to every new/changed public API, invariant-enforcing function, error type, external contract integration, and any test covering a TV-<nnn>.

PROCEDURE

1. Determine the next minimal, self-contained set of tasks to implement (small chunk) from the Reviewer Feedback and the Delivery Plan for the next incomplete S-id. Consult ${BLUEPRINTS_DIR}/03-contracts.md for any concrete types/endpoints that must be implemented.
2. Use TDD where feasible: add/adjust tests for referenced TVs, then implement code to satisfy them.
   2a. Add/maintain doc-comment tags: `@impl/@s/@contract` on implementation items; `@covers/@tv` on tests.
3. As you implement, update the module's docs/ with usage examples and basic information for any changed surface. Keep details in code doc comments and avoid duplicating them in docs/. Do not edit Delivery Plan text.
4. Before starting the next checklist item, mark every completed task as `[x]` in ${BLUEPRINTS_DIR}/05-delivery-plan.md.
   4a. If you discover an unchecked Delivery Plan item that already appears implemented, verify conformance to Spec/Contracts/Test Vectors (APIs, invariants, error contracts, and required tests/tags). If it passes, check it off; otherwise, fix the implementation and then check it off.
5. Keep changes tight and idempotent. Avoid unrelated edits.
   5a. Keep source files small: aim for under ~300 lines per file where possible; split into smaller files based on logical boundaries (cohesive responsibilities) using modules/submodules as appropriate.
6. Before emitting output, verify that every checklist item in ${BLUEPRINTS_DIR}/05-delivery-plan.md is checked. Reconcile any mismatches by verifying and checking off items that are already implemented (or fixing the implementation first). If any remain unchecked, do not emit ${COMPLETED_TOKEN}; emit ${CONTINUE_TOKEN} with a numbered remaining-work list.
7. When reviewer requests removals, delete the specified scopes (files, functions/methods, blocks/lines). Remove resulting dead references (imports/exports, type/usages, feature flags/config, tests/docs) and any now-unused dependencies. Do not leave parked/disabled code without a corresponding Delivery Plan item.

RUN CONTROL

- Limit scope: a small number of files/tasks per iteration.
- If a hard precondition prevents progress (e.g., required inputs missing), output exactly ${ERROR_TOKEN}.
- Output exactly ${COMPLETED_TOKEN} only when both are true:
  1. All planned work and reviewer feedback are fully addressed, and
  2. Every checklist item in ${BLUEPRINTS_DIR}/05-delivery-plan.md is marked `[x]` (no unchecked items remain).
- Otherwise, output exactly ${CONTINUE_TOKEN} followed by a numbered list of the remaining work to do next.

OUTPUT RULES

- Do not print explanations or logs beyond the token and (for continue) the remaining-work list.
- Do not edit ${BLUEPRINTS_DIR}/01-requirements.md, ${BLUEPRINTS_DIR}/02-spec.md, ${BLUEPRINTS_DIR}/03-contracts.md, or ${BLUEPRINTS_DIR}/04-test-vectors.md contents.

REVIEWER_FEEDBACK_OR_REMAINING_WORK:
${REVIEWER_FEEDBACK_OR_REMAINING_WORK}

Upon completion, print exactly one of: ${COMPLETED_TOKEN} | ${CONTINUE_TOKEN} | ${ERROR_TOKEN}.
