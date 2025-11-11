# Build Minimal, Machine-Readable Test Vectors

GOAL
- Apply the Implementation Plan to create/update ${BLUEPRINTS_DIR}/04-test-vectors.md using a compact, line-based format for MVP coverage.

INPUTS (read-only)
- Spec: ${BLUEPRINTS_DIR}/02-spec.md
- Requirements: ${BLUEPRINTS_DIR}/01-requirements.md
- Contracts (may be missing or empty): ${BLUEPRINTS_DIR}/03-contracts.md (handwritten Markdown with `C-###` items)
- Test Vectors (may be missing): ${BLUEPRINTS_DIR}/04-test-vectors.md
- Implementation Plan: between ---PLAN START--- and ---PLAN END--- below

PARSING (fast, exact)
- Requirements: lines matching `^R-[0-9]{3} - (.+)[.!?]$`; extract R-id and text; preserve numeric order.
- Spec: records-only S-lines matching `^S-([0-9]{3}(\.[0-9]+)?) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| DO:(.+?) \| TITLE:([^|]+)( \| .+)?$` (optional IF/ER/LM/OB fields may follow); extract S-id and R-id list; preserve file order.
- TITLE is mandatory and must appear immediately after DO:, before any optional IF/ER/LM/OB fields. Keep TITLE a short, stable phrase reused verbatim downstream.
- Ignore non-matching lines; ASCII only; no tabs.

FILE FORMAT (strict, records-only)
- ASCII only. No headers/sections/tables. No tabs. One test per line.
- Separator: " | " (space, pipe, space). Fixed uppercase keys.
- Line schema (minimal):
  TV-### | R:R-###[,R-###...] | S:S-###[.n][,S-###[.n]...] | L:<one or more of U,I,P comma-separated> | GIVEN:<preconditions> | WHEN:<inputs/actions> | THEN:<expected>
- L codes legend (interpreted by the Delivery Plan workflow):
  - U: Unit-level or isolated component behavior
  - I: Integration or cross-component/IO interactions (flags integration tasks)
  - P: Property-based behavior (flags proptest scaffolding)
- Optional fields (append only when needed): | ERR:<errors> | DET:<determinism> | OB:<observability>
- Validation regex (core): ^TV-([0-9]{3}) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| S:(S-[0-9]{3}(\.[0-9]+)?(,S-[0-9]{3}(\.[0-9]+)?)*) \| L:([UIP](,[UIP])*) \| GIVEN:.+ \| WHEN:.+ \| THEN:.+
- IDs: TV-### are zero-padded, unique, strictly increasing. Append using the next integer; never renumber existing.

POLICY (MVP-first)
- Prefer the smallest set of vectors validating core behaviors and critical boundaries; defer exhaustive edge cases.
- Keep statements concise and unambiguous; include determinism only when essential.

ACTIONS
- Referential integrity (mandatory, pre-apply): for every plan line, ensure all referenced S-ids exist in Spec and all referenced R-ids exist in Requirements. Additionally, for each S-id on the plan line, enforce that the plan line's R-id list is a subset of that S clause's `R:` coverage in the Spec. If any check fails, or if Requirements or Spec are missing/empty, do not modify files; print exactly: ${ERROR_TOKEN}.
- For each plan line:
  - ADD | R:... | S:... | L:... | GIVEN:... | WHEN:... | THEN:...[ | ERR:...][ | DET:...][ | OB:...]
    -> Append a new TV line with the next TV-###.
  - REPLACE TV-### | R:... | S:... | L:... | GIVEN:... | WHEN:... | THEN:...[ | ...]
    -> Replace the entire line for that TV-###.
  - REMOVE TV-###
    -> Delete that line; if TV-### does not exist, perform a no-op (idempotent).
- Create ${BLUEPRINTS_DIR} if missing. Create the file if missing.
- Ensure idempotency: deduplicate; keep lines sorted ascending by TV id; validate against the regex and fix minor spacing.
- Ordering semantics (clarified):
  - Test Vector file is always sorted by `TV-###` ascending.
  - Reviewer emits plan lines in Spec S-id order; builder may apply in any order but must preserve TV file sorted by TV id.
  - Within each TV line, list `S:` and `R:` ids in ascending numeric order.

OUTPUT
- If Spec is missing/empty, or referential integrity fails, print exactly: ${ERROR_TOKEN}
- Otherwise, after applying all actions, print exactly: ${COMPLETED_TOKEN}

BEGIN IMPLEMENTATION
---PLAN START---
${IMPLEMENTATION_PLAN}
---PLAN END---
