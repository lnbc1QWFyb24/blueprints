# Blueprints Canonical Rules

- Canonical reference: See BLUEPRINTS.md (inlined below). Treat as authoritative for formats, IDs, and cross-references.
- Enforce strict compliance with file schemas and ID discipline across Requirements (R-###), Spec (S-###[.n]), Contracts (C-###), Test Vectors (TV-###), Delivery Plan, and Lifecycle.
- Prefer correctness over convenience: when instructions conflict with blueprints rules, fix the content or request clarification; do not compromise the rules.

---

# Blueprints File Types

- Requirements (`01-requirements.md`): one-line records, IDs `R-###` describing WHAT/WHY.
- Spec (`02-specs.md`): strict clause lines `S-###(.n)` with `R:` references and `DO`, required `TITLE`, and optional `IF/ER/LM/OB` in canonical order.
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

# Additional Spec Rules

- TITLE is mandatory: 4–7 words, ASCII, appears immediately after `DO:`; reused verbatim for Delivery Plan milestones.
- Coverage lines (exemptions): `COVERAGE | R:R-###[,R-###...] | REASON:<short>`; disjoint from any `S-*` referencing the same `R-###`.
- Creation-stage policy: during initial spec creation and upon explicit approval, you may delete/rewrite and then renumber sequentially from `S-001` (preserving `.n`); thereafter append only.
