# Build Minimal, Machine-Readable Test Vectors

GOAL
- Apply the Implementation Plan to create/update `${BLUEPRINTS_DIR}/04-test-vectors.md` using a compact, line-based format for MVP coverage.

INPUTS (read-only)
- Spec: `${BLUEPRINTS_DIR}/02-specs.md`
- Requirements: `${BLUEPRINTS_DIR}/01-requirements.md`
- Contracts (may be missing or empty): `${BLUEPRINTS_DIR}/03-contracts.md` (handwritten Markdown with `C-###` items)
- Test Vectors (may be missing): `${BLUEPRINTS_DIR}/04-test-vectors.md`
- Implementation Plan: between ---PLAN START--- and ---PLAN END--- below

PARSING (fast, exact)
- Requirements: lines matching `^R-[0-9]{3} - (.+)[.!?]$`; extract R-id and text; preserve numeric order.
- Spec: records-only S-lines matching `^S-([0-9]{3}(\\.[0-9]+)?) \\| R:(R-[0-9]{3}(,R-[0-9]{3})*) \\| DO:(.+?) \\| TITLE:([^|]+)( \\| .+)?$` (optional IF/ER/LM/OB fields may follow); extract S-id and R-id list; preserve file order.
- TITLE is mandatory and appears immediately after DO:.
- Ignore non-matching lines; ASCII only; no tabs.

FILE FORMAT (strict, records-only)
- ASCII only. No headers/sections/tables. No tabs. One test per line.
- Separator: ` | ` (space, pipe, space). Fixed uppercase keys.
- Line schema (minimal):
  TV-### | R:R-###[,R-###...] | S:S-###[.n][,S-###[.n]...] | L:<one or more of U,I,P comma-separated> | GIVEN:<preconditions> | WHEN:<inputs/actions> | THEN:<expected>
- Optional fields (append only when needed): | ERR:<errors> | DET:<determinism> | OB:<observability>
- Validation regex (core): `^TV-([0-9]{3}) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| S:(S-[0-9]{3}(\\.[0-9]+)?(,S-[0-9]{3}(\\.[0-9]+)?)*) \| L:([UIP](,[UIP])*) \| GIVEN:.+ \| WHEN:.+ \| THEN:.+`
- IDs: TV-### zero-padded, strictly increasing. Append using the next integer; never renumber existing.

POLICY (MVP-first)
- Prefer the smallest set of vectors validating core behaviors and critical boundaries; defer exhaustive edge cases.
- Keep statements concise and unambiguous; include determinism only when essential.

ACTIONS
- Referential integrity (mandatory, pre-apply): for every plan line, ensure all referenced S-ids exist in Spec and all referenced R-ids exist in Requirements. Additionally, for each S-id on the plan line, enforce that the plan line's R-id list is a subset of that S clause's `R:` coverage in the Spec. If any check fails, or if Requirements or Spec are missing/empty, abort without modifying files and return a concise error.
- For each plan line:
  - `ADD | R:... | S:... | L:... | GIVEN:... | WHEN:... | THEN:...[ | ERR:...][ | DET:...][ | OB:...]` → Append a new TV line with the next TV-###.
  - `REPLACE TV-### | R:... | S:... | L:... | GIVEN:... | WHEN:... | THEN:...[ | ...]` → Replace the entire line for that TV-###.
  - `REMOVE TV-###` → Delete that line; if TV-### does not exist, perform a no-op.
- Create `${BLUEPRINTS_DIR}` if missing. Create the file if missing.
- Ensure idempotency: deduplicate; keep lines sorted ascending by TV id; validate against the regex and fix minor spacing.
- Ordering: Test Vector file is always sorted by `TV-###` ascending; within each TV line, list `S:` and `R:` ids in ascending numeric order.

OUTPUT
- Apply actions and finish without emitting special control tokens; if a precondition fails, abort without changes and return a concise error.

BEGIN IMPLEMENTATION
---PLAN START---
${IMPLEMENTATION_PLAN}
---PLAN END---
