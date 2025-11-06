# Production Code Quality

- No placeholders or stubs in production code: avoid `TODO`, `FIXME`, `XXX`, `HACK`, `unimplemented!()`, `todo!()`, or panic placeholders.
- No temporary diagnostics or debug remnants in production: `dbg!`, `eprintln!`, commented-out blocks, or disabled/feature-flagged code without a Delivery Plan item.
- Avoid `unwrap/expect` in production paths; use explicit error handling or document unreachable invariants with rationale.
- Implement error handling, invariants, and edge cases; avoid happy-path-only logic.
- Ensure naming, visibility, and module structure match Spec/Contracts; do not leak internals unintentionally.

---

# File Size & Modularity

- Aim for files under ~300 lines where possible.
- Split larger files into smaller modules along logical boundaries (cohesive responsibilities).
- Remove dead code and unused dependencies when scope changes; avoid parked/disabled code without a Delivery Plan item.

---

# Traceability Tags & `sg` Queries

- Implementations (functions, methods, types, trait impls): `/// @impl(R-...[,R-...])`; optionally `/// @s(S-...[,S-...])`; and `/// @contract(C-###)` when applicable.
- Tests (`#[test]` functions): `/// @covers(R-...[,R-...])` and `/// @tv(TV-...)`.
- Optional module roll-up: `//! @s(S-...)` on module roots when helpful.

Examples (`sg`):

- Implementations: `sg --lang rust -p '#[doc = "@impl(R-017)"]'`
- Tests covering a requirement: `sg --lang rust -p '#[test] #[doc = "@covers(R-017)"]'`
- Tests for a vector: `sg --lang rust -p '#[doc = "@tv(TV-015)"]'`
- Contract touch points: `sg --lang rust -p '#[doc = "@contract(C-001)"]'`

Coverage requirement: apply tags to every new/changed public API, invariant-enforcing function, error type, external contract integration, and any test covering a TV-<nnn>.
