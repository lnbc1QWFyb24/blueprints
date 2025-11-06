# Workspace Constraints

- All paths are relative to the current working directory (CWD).
- Do not `cd`; do not read or write outside CWD.
- Assume `${BLUEPRINTS_DIR}` exists within the workspace; resolve all blueprints paths relative to it.

- Do not create or modify Cargo workspace members (no new crates/packages, no workspace edits). Work strictly within the target crate.
- Target crate: `${CRATE_NAME}` rooted at `${CRATE_ROOT}`. Limit edits to files under this crate.
- If `${MODULE_REL_PATH}` is provided, keep implementation changes focused under `${CRATE_ROOT}/${MODULE_REL_PATH}` and the crate’s related tests/docs. Do not move or rename the module unless explicitly requested.
