# Blueprints Canonical Rules

- Canonical reference: See BLUEPRINTS.md (inlined below). Treat as authoritative for formats, IDs, and cross-references.
- Enforce strict compliance with file schemas and ID discipline across Requirements (R-###), Spec (S-###[.n]), Contracts (C-###), Test Vectors (TV-###), Delivery Plan, and Lifecycle.
- Prefer correctness over convenience: when instructions conflict with blueprints rules, fix the content or request clarification; do not compromise the rules.

---

# Blueprints File Types

- Requirements (`01-requirements.md`): one-line records, IDs `R-###` describing WHAT/WHY.
- Spec (`02-specs.md`): strict clause lines `S-###(.n)` with `R:` references and `DO/IF/ER/LM/OB` as needed; `TITLE:` field is mandatory.
- Contracts (`03-contracts.md`): heading entries `### C-### — <Short Title>` with precise Rust types, external APIs (METHOD PATH), auth, request/response mapping, error types/codes, and doc links.
- Test Vectors (`04-test-vectors.md`): concrete, deterministic cases referencing S-ids; include R and S references.
- Delivery Plan (`05-delivery-plan.md`): actionable milestones and checklist items referencing R/S/C/TV.
- Lifecycle (`06-lifecycle.md`): append-only status ledger for `R/S/S.n/TV/C` using the ledger schema.

---

# Blueprints Scope & Files

- Blueprints directory: `${BLUEPRINTS_DIR}`
- Files in scope:
  - `${BLUEPRINTS_DIR}/01-requirements.md`
  - `${BLUEPRINTS_DIR}/02-specs.md`
  - `${BLUEPRINTS_DIR}/03-contracts.md`
  - `${BLUEPRINTS_DIR}/04-test-vectors.md`
  - `${BLUEPRINTS_DIR}/05-delivery-plan.md`
  - `${BLUEPRINTS_DIR}/06-lifecycle.md`

# Lifecycle Ledger Format

- Ledger path: `${BLUEPRINTS_DIR}/06-lifecycle.md` (append-only).
- Record schema:
  - `<ID> | STATUS:<active|deprecated|removed> | REASON:<short>[ | EFFECTIVE:<semver|date>][ | REPLACE_BY:<ID>]`
- IDs may be `R-###`, `S-###` or `S-###.n`, `TV-###`, or `C-###`.
- For deprecations/removals, always append a new ledger record rather than mutating prior entries.

---

# Spec Rules (strict)

- S-line schema (mandatory TITLE, canonical order):
  - `S-###[.n] | R:R-###[,R-###...] | DO:<imperative, testable statement> | TITLE:<concise, stable title>[ | IF:<apis/types>][ | ER:<errors>][ | LM:<limits>][ | OB:<observability>]`
- Validation regex:
  - `^S-([0-9]{3}(\.[0-9]+)?) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| DO:([^|]+?) \| TITLE:([^|]+)( \| IF:[^|]+)?( \| ER:[^|]+)?( \| LM:[^|]+)?( \| OB:[^|]+)?$`
- TITLE conventions: ASCII-only, short and stable (4–7 words), reused verbatim downstream, must appear immediately after `DO:`.
- COVERAGE lines for exemptions:
  - `COVERAGE | R:R-###[,R-###...] | REASON:<short>`
  - Disjointness: an `R-###` appears either in some `S-*` or in a `COVERAGE` line, not both.
- Renumbering policy: During the Design command/prompt, renumbering is fully allowed (you may rewrite and renumber sequentially from `S-001`, preserving `.n`); the Update command/prompt must not renumber and should append only.

---

# Requirements Rules (strict)

- Records-only; ASCII; one line per requirement; no headers/tabs.
- Line schema and validation:
  - `R-### - <One-sentence description of WHAT and WHY>`
  - Regex: `^R-(\d{3})\s-\s[\x20-\x7E]+[.!?]$` (ASCII printable only; ends with `.`, `!`, or `?`).
- IDs: zero-padded ascending `R-###`; unique.
- Renumbering policy:
  - Design phase: allowed to rewrite and renumber sequentially from `R-001` after triage approval when consolidating; delete/defer as needed to minimize MVP.
  - Update/Implement phases: do not renumber; append or edit in place while preserving existing IDs and ordering.
