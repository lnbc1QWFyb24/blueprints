# Objective

- Implement the Delivery Plan end-to-end while incorporating the latest Reviewer feedback.
- Make focused, small, verifiable changes (scoped chunks) per iteration.

---

# Inputs

- Spec (authoritative; read-only): `${BLUEPRINTS_DIR}/02-specs.md`
- Contracts (read-only): `${BLUEPRINTS_DIR}/03-contracts.md` (heading entries `### C-### — <Title>`)
- Test Vectors (read-only): `${BLUEPRINTS_DIR}/04-test-vectors.md` (records-only TV lines)
- Delivery Plan: `${BLUEPRINTS_DIR}/05-delivery-plan.md` (follow the Delivery Plan section)
- Requirements (read-only): `${BLUEPRINTS_DIR}/01-requirements.md`
- blueprints (authoritative; read-only) — see BLUEPRINTS.md (inlined below)
- Reviewer Feedback (numbered list) and/or Remaining Work List (from a prior builder iteration).

---

# Target Scope

- Crate: `${CRATE_NAME}` at `${CRATE_ROOT}`.
- Scope: modify files only within this crate. Do not create new crates or change workspace members.
- Focus: if `${MODULE_REL_PATH}` is set, center changes under `${CRATE_ROOT}/${MODULE_REL_PATH}` plus related tests/docs within the crate.

---

# Procedure

1. Determine the next minimal, self-contained set of tasks to implement (small chunk) from the Reviewer Feedback and the Delivery Plan for the next incomplete S-id. Consult `${BLUEPRINTS_DIR}/03-contracts.md` for any concrete types/endpoints that must be implemented.
2. Use TDD where feasible: add/adjust tests for referenced TVs, then implement code to satisfy them.
   2a. Apply required traceability tags on implementations and tests, using the Implementation Standards section.
3. As you implement, update the module's `docs/` with usage examples and basic information for any changed surface. Keep details in code doc comments and avoid duplicating them in docs/. Follow the Delivery Plan section when deciding what to document and mark complete.
4. Before starting the next checklist item, mark every completed task as `[x]` in `${BLUEPRINTS_DIR}/05-delivery-plan.md` only after verifying the Checklist completion criteria.
   4a. If you discover an unchecked Delivery Plan item that already appears implemented, verify conformance to Spec/Contracts/Test Vectors (APIs, invariants, error contracts, and required tests/tags). If it passes the criteria, check it off; otherwise, fix the implementation and then check it off.
   4b. If any already-checked item fails the criteria (e.g., placeholders, missing tests/docs/tags), uncheck it (`[ ]`) and add the missing work to the remaining-work list.
5. Keep changes tight and idempotent. Avoid unrelated edits.
   5a. Keep source files small and modular, following the Implementation Standards section.
6. Before emitting output, verify that every checklist item in `${BLUEPRINTS_DIR}/05-delivery-plan.md` is checked and meets the Delivery Plan Checklist Criteria from the Delivery Plan section. Reconcile any mismatches by verifying and checking off items that are already implemented (or fixing the implementation first). If any remain unchecked or fail the criteria, do not emit `${COMPLETED_TOKEN}`; emit `${CONTINUE_TOKEN}` with a numbered remaining-work list.
7. When reviewer requests removals, delete the specified scopes (files, functions/methods, blocks/lines). Remove resulting dead references (imports/exports, type/usages, feature flags/config, tests/docs) and any now-unused dependencies. Do not leave parked/disabled code without a corresponding Delivery Plan item.

---

# Deliverables

- Code matching Spec APIs/invariants and error contracts, plus concrete Contracts (types and external integrations) defined in `03-contracts.md` where applicable.
- Tests that implement relevant TV-<nnn> for the targeted S-ids; apply required tags on tests and implementations per the Implementation Standards section.
- Docs: maintain the module's documentation under its `docs/` directory (within the target module). Provide usage examples and basic information; detailed behavior belongs in code doc comments. Align with the Delivery Plan section.
- Delivery Plan: check off completed tasks only; do not rename/remove.

---

# Review Loop Variable and Tokens

REVIEWER_FEEDBACK_OR_REMAINING_WORK:
${REVIEWER_FEEDBACK_OR_REMAINING_WORK}

Use the Tokens & Output Protocol section for token emission.
