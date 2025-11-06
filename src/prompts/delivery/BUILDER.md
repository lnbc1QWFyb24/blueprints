# Generate and Maintain Delivery Plan (TDD-first)

## Objective

Create and maintain an actionable, checklist-driven `${BLUEPRINTS_DIR}/05-delivery-plan.md` that fully covers the Spec, the human-written Contracts doc, and the Test Vectors with explicit TDD sequencing. The plan must begin with implementing the tests for the Test Vectors (acknowledging they will initially fail), followed by implementation tasks that progressively make those tests pass.

## Inputs

- All paths are relative to the current working directory (CWD). Do not `cd`; do not read or write outside CWD.
- **Primary Spec**: `${BLUEPRINTS_DIR}/02-spec.md` (authoritative HOW; do not edit)
- **Contracts (read-only, may be missing or empty)**: `${BLUEPRINTS_DIR}/03-contracts.md` (handwritten Markdown with `C-###` items: concrete Rust types, external APIs, links, and notes)
- **Test Vectors**: `${BLUEPRINTS_DIR}/04-test-vectors.md` (canonical, records-only TV lines; do not edit)
- **Requirements (read-only)**: `${BLUEPRINTS_DIR}/01-requirements.md` (for context; do not edit)

## File Contract

- If `${BLUEPRINTS_DIR}/05-delivery-plan.md` does not exist, **create it**.
- Maintain idempotency: re-runs refine/append/update without duplicating checklist items or milestones.
- Preserve and extend existing content; do not delete valid coverage unless superseded for clarity/correctness.

## Coverage Rules

- **Traceability**: Every Spec clause `S-<id>` must be represented by a Milestone with checklist items; every Test Vector `TV-<nnn>` must have a corresponding test implementation task.
- **Contracts Coverage**: When `${BLUEPRINTS_DIR}/03-contracts.md` exists, ensure implementation tasks explicitly align with the types and external endpoints described there (names, method+path, auth scheme, request/response shapes, and error mapping). Prefer placing these under the corresponding S-id milestone(s) that reference or exercise them, and cite the relevant `C-###` item in task text when available.
- **TV Presence per S-id**: Each `S-<id>` must have at least one referencing `TV-<nnn>` recorded in `${BLUEPRINTS_DIR}/04-test-vectors.md`.
  - If none exist for a given `S-<id>`, the builder must:
    - Create/maintain the milestone for that `S-<id>`.
    - Do not invent tests. Add exactly one blocking placeholder checklist item under Implementation Tasks: `- [ ] DP-<nnn> Add minimal test vector(s) for S-<id> via ./test-vectors.sh (default L:U); Refs: S-<id>`
    - Set the Traceability Table `Status` to `Missing` for that `S-<id>`.
    - Include a concise note in `Notes & Risks`: `Blocking: no test vectors for S-<id>; delivery plan remains incomplete until vectors are added.`
  - This avoids oscillation by providing a structured placeholder until TVs are authored.
  - **Completeness per S-id**: For each `S-<id>`, ensure:
    - One Milestone titled `S-<id>: <Spec Title>`.
      - Spec must include a concise, stable `TITLE:` field (short phrase) immediately after `DO:` for every S-line; use this `TITLE` verbatim for milestone titles.
    - A TDD block listing test tasks for all `TV-<nnn>` referencing that S-id. Each test task must explicitly state: "Tests will fail initially until implementation is complete."
    - Implementation tasks to satisfy the Spec clause, referencing the exact S-ids and any Spec-defined APIs, invariants, and error codes.
    - Additional tasks as applicable: Integration glue, Proptest scaffolding, invariants/assertions, error handling, and docs updates driven by the Spec (not Requirements).
- **DP-ids**: Each checklist item must have a unique `DP-<nnn>` id, zero-padded to width 3 (e.g., `DP-001`), strictly increasing, and unique across the entire plan (global scope, not per milestone). Always continue from the current maximum.
- **No Orphans**: Every `TV-<nnn>` from `${BLUEPRINTS_DIR}/04-test-vectors.md` must appear in at least one test task.

## Plan Schema

Use exactly this structure for each S-id milestone (TVs have no titles):

```
## Milestone: S-<id>: <Spec Title>

Refs: S-<id>[, S-<id2> ...]

TDD — Test Tasks (will fail initially):
- [ ] DP-<nnn> Implement test: TV-<nnn>; Refs: S-<id>, TV-<nnn>
- [ ] DP-<nnn> Implement test: TV-<nnn2>; Refs: S-<id>, TV-<nnn2>

Implementation Tasks:
- [ ] DP-<nnn> Implement <module/function/API> per Spec; Refs: S-<id>
- [ ] DP-<nnn> Enforce invariants/error codes when defined in Spec; Refs: S-<id> (include only if applicable)
- [ ] DP-<nnn> Add integration wiring / IO handling; Refs: S-<id> (include only if cross-component or any referencing TV has `L` containing `I`)
- [ ] DP-<nnn> Add property-based test scaffolding; Refs: S-<id> (include only if Spec/TVs indicate property behavior, e.g., any referencing TV has `L` containing `P`)
- [ ] DP-<nnn> Docs/README update (API/usage examples) or mark exactly "No doc change required"; Refs: S-<id> (hard requirement: when skipping docs edits, include this exact literal phrase)

Notes & Risks:
- <Concise notes: edge cases, sequencing, parallel safety, migrations>
```

Include a top-level TDD guidance note stating: "Start by writing the tests for all referenced Test Vectors; expect red (failing) tests. Implement code iteratively until tests go green, S-id by S-id." If Contracts exist, add a global note to align types and endpoints to `${BLUEPRINTS_DIR}/03-contracts.md`.

## Document Layout

1. **Header** with purpose and linkage to Spec and Test Vectors, including the TDD-first philosophy.
2. **Traceability Table** (maintained each run):
   - Columns: `S-id | Title | TV Count | Test Tasks | Impl Tasks | Integration | Proptest | Status`
   - Column semantics:
     - `Title` comes from the mandatory Spec `TITLE:` field (immediately after `DO:`). Do not derive aliases.
     - `Test Tasks` and `Impl Tasks` are integer counts of checklist items present in the milestone.
     - `Integration` is `Yes` if any referencing TV for the S-id has `L` containing `I` or the milestone includes at least one integration task; otherwise `No`.
     - `Proptest` is `Yes` if any referencing TV for the S-id has `L` containing `P` or the milestone includes at least one proptest task; otherwise `No`.
     - `Status` is exactly `Covered` only if TV Count ≥ 1, every TV for the S-id has a corresponding test task, and there is ≥1 implementation task; otherwise exactly `Missing`.
3. **Milestones**: one milestone section per S-id, in Spec order, following the schema above.
4. **Global Risks & Notes** (optional if none).

## Procedure

1. **Locate Spec, Contracts, and Test Vectors**:
   - Require `${BLUEPRINTS_DIR}/02-spec.md` and `${BLUEPRINTS_DIR}/04-test-vectors.md`. If either file is missing or empty, halt and output exactly `${ERROR_TOKEN}`.
   - Parse Spec as records-only S-lines using regex (optional IF/ER/LM/OB fields may follow): `^S-([0-9]{3}(\.[0-9]+)?) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| DO:(.+?) \| TITLE:([^|]+)( \| .+)?$`
     - Extract: S-id, R-id list, DO text, and required TITLE text. For `Title`, use the `TITLE` exactly as written.
   - From `${BLUEPRINTS_DIR}/04-test-vectors.md`, parse records-only TV lines using regex (multi-value `L` supported, comma-separated): `^TV-([0-9]{3}) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| S:(S-[0-9]{3}(\.[0-9]+)?(,S-[0-9]{3}(\.[0-9]+)?)*) \| L:([UIP](,[UIP])*) \| GIVEN:.+ \| WHEN:.+ \| THEN:.+`
     - Extract: TV id, S-id list, R-id list, and Level set `L` (e.g., `U`, `I`, `P`, or `I,P`).
  - If present, skim `${BLUEPRINTS_DIR}/03-contracts.md` (Markdown with `C-###` items). Extract practical cues:
    - Rust type names and example code blocks.
    - External API endpoints (METHOD and PATH), auth schemes, and linked docs.
    - Error mappings, retry/backoff, pagination, and observability hints.
    Use these to add or refine integration and type implementation tasks under relevant S-id milestones.
2. **Load/Create Delivery Plan File**:
   - If missing, scaffold with Header, empty Traceability Table, and milestone anchors per S-id with TDD guidance note.
3. **Iterate S-ids (one at a time)**:
   - Find existing milestone for the S-id.
   - Ensure test tasks exist for every `TV-<nnn>` that references this S-id; if missing, add them with explicit "will fail initially" wording.
   - If there are no referencing `TV-<nnn>` for this S-id, add the single blocking placeholder implementation checklist item described in Coverage Rules (do not add test tasks), flag Status `Missing`, and add a brief note under `Notes & Risks`.
   - Ensure implementation tasks exist covering core logic, invariants, errors, and any integration/proptest necessities implied by the Spec or vectors.
   - Maintain unique, increasing `DP-<nnn>` ids (global across the file); do not duplicate tasks. Detect the current maximum DP id and continue numbering from there.
4. **Update Traceability Table** for the S-id reflecting counts and Status per rules above.
5. **Finalize**:
   - Write back `${BLUEPRINTS_DIR}/05-delivery-plan.md`.
   - If and only if every S-id is `Covered` in the Traceability Table, output exactly `${COMPLETED_TOKEN}`.
   - Otherwise, output exactly `${CONTINUE_TOKEN}`.

## Scope of Work

- Load the Spec and all Test Vectors.
- Create or update the Delivery Plan to globally cover every S-id and TV-id in a single pass where possible.
- Add or repair all missing milestones, checklist items, and traceability rows to achieve full coverage and idempotency.

## Output Discipline

- Do not modify `${BLUEPRINTS_DIR}/01-requirements.md`, `${BLUEPRINTS_DIR}/02-spec.md`, or `${BLUEPRINTS_DIR}/04-test-vectors.md`.
- Only write to `${BLUEPRINTS_DIR}/05-delivery-plan.md`.
- Upon completion of this run, print `${COMPLETED_TOKEN}` if the plan fully covers all S-ids and TV-ids; otherwise print `${CONTINUE_TOKEN}` if further iterations are needed.

---

Apply all reviewer feedback items exhaustively and idempotently to update `${BLUEPRINTS_DIR}/05-delivery-plan.md`.

Reviewer feedback (verbatim):
${REVIEWER_FEEDBACK}

Upon completion, print exactly one of: ${COMPLETED_TOKEN} | ${CONTINUE_TOKEN} | ${ERROR_TOKEN}.
