# Generate and Maintain Delivery Plan (TDD-first)

Objective

- Create and maintain an actionable, checklist-driven `${BLUEPRINTS_DIR}/05-delivery-plan.md` that fully covers the Spec, the human-written Contracts doc, and the Test Vectors with explicit TDD sequencing.
- Begin with implementing the tests for the Test Vectors (initially failing), followed by implementation tasks that progressively make those tests pass.

Inputs

- Spec (authoritative): `${BLUEPRINTS_DIR}/02-specs.md`
- Contracts (read-only, may be missing or empty): `${BLUEPRINTS_DIR}/03-contracts.md`
- Test Vectors: `${BLUEPRINTS_DIR}/04-test-vectors.md`
- Requirements (read-only): `${BLUEPRINTS_DIR}/01-requirements.md`

File Contract

- If `${BLUEPRINTS_DIR}/05-delivery-plan.md` does not exist, create it.
- Maintain idempotency: re-runs refine/append/update without duplicating items or milestones.
- Preserve and extend existing content; do not delete valid coverage unless superseded for clarity/correctness.

Coverage Rules

- Traceability: Every Spec clause `S-<id>` must be represented by a Milestone with checklist items; every Test Vector `TV-<nnn>` must have a corresponding test implementation task.
- Contracts Coverage: when `${BLUEPRINTS_DIR}/03-contracts.md` exists, align implementation tasks to the types and endpoints described there; cite `C-###` where applicable.
- TV Presence per S-id: Each `S-<id>` must have at least one referencing `TV-<nnn>`. If none exist:
  - Create the milestone and add exactly one blocking placeholder implementation checklist item: `- [ ] DP-<nnn> Add minimal test vector(s) for S-<id> via ./test-vectors.sh (default L:U); Refs: S-<id>`.
  - Set Status `Missing` for that `S-<id>` and include a brief `Blocking` note.
- Completeness per S-id: Ensure one Milestone titled `S-<id>: <Spec Title>` with:
  - TDD — Test Tasks for all referencing `TV-<nnn>` stating: "Tests will fail initially until implementation is complete."
  - Implementation Tasks covering core logic, invariants/errors, integration wiring (if any TV has `L` containing `I`), proptest scaffolding (if any TV has `L` containing `P`), and docs updates or the explicit literal phrase `No doc change required`.
- DP-ids: `DP-<nnn>` unique, zero-padded to width 3, strictly increasing globally across the file.
- No Orphans: Every `TV-<nnn>` must appear in at least one test task.

Plan Schema

Header
- Purpose and linkage to Spec and Test Vectors; include TDD-first philosophy: "Start by writing the tests for all referenced Test Vectors; expect red (failing) tests. Implement code iteratively until tests go green, S-id by S-id." If Contracts exist, add a global note to align to `${BLUEPRINTS_DIR}/03-contracts.md`.

Traceability Table (maintained each run)
- Columns: `S-id | Title | TV Count | Test Tasks | Impl Tasks | Integration | Proptest | Status`
- Semantics:
  - Title from mandatory Spec `TITLE:` field.
  - Test/Impl tasks are counts of checklist items in the milestone.
  - Integration `Yes` if any referencing TV has `L` containing `I` or milestone includes integration tasks; else `No`.
  - Proptest `Yes` if any referencing TV has `L` containing `P` or milestone includes proptest tasks; else `No`.
  - Status `Covered` only if TV Count ≥ 1, every TV has a test task, and there is ≥1 implementation task; else `Missing`.

Milestones (one per S-id)

```
## Milestone: S-<id>: <Spec Title>

Refs: S-<id>[, S-<id2> ...]

TDD — Test Tasks (will fail initially):
- [ ] DP-<nnn> Implement test: TV-<nnn>; Refs: S-<id>, TV-<nnn>

Implementation Tasks:
- [ ] DP-<nnn> Implement <module/function/API> per Spec; Refs: S-<id>
- [ ] DP-<nnn> Enforce invariants/error codes when defined in Spec; Refs: S-<id>
- [ ] DP-<nnn> Add integration wiring / IO handling; Refs: S-<id>
- [ ] DP-<nnn> Add property-based test scaffolding; Refs: S-<id>
- [ ] DP-<nnn> Docs/README update (API/usage examples) or mark exactly "No doc change required"; Refs: S-<id>

Notes & Risks:
- <Concise notes: edge cases, sequencing, parallel safety, migrations>
```

Procedure

1) Load Spec, Contracts (if present), and Test Vectors; if Spec or Test Vectors are missing or empty, abort without modifying files and stop.
2) Ensure file exists/scaffolded with Header and a blank Traceability Table; maintain idempotency.
3) For each S-id in Spec order, ensure a milestone exists and is populated per rules; ensure DP-ids continue from current max.
4) Update the Traceability Table to reflect counts and Status per S-id.
5) Write back `${BLUEPRINTS_DIR}/05-delivery-plan.md`.

Output Discipline

- Do not modify `${BLUEPRINTS_DIR}/01-requirements.md`, `${BLUEPRINTS_DIR}/02-specs.md`, or `${BLUEPRINTS_DIR}/04-test-vectors.md`.
- Only write to `${BLUEPRINTS_DIR}/05-delivery-plan.md`.
- Do not emit special control tokens; complete the plan update. If some S-ids remain `Missing`, include that status in the Traceability Table.
