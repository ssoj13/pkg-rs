# Plan 11 - Full Rust Port + Python Minimization

Date: 2026-02-08

## Target State
- Rust owns config, resolver, context, build, pip, repo backends, CLI, and plugins.
- Embedded Python remains only for:
  - executing `package.py`
  - reading `rezconfig.py` (optional compatibility mode)
  - running pip/uv subprocesses when required (no Rez modules).
- `python/` tree shrinks to a minimal bootstrap (no Rez source, no rezplugins, no rezgui).

## Constraints
- Must preserve Rez-compatible inputs/outputs for build/context/pip.
- Single binary (`pkg`) keeps Rez command parity (`pkg rez *`).
- No dependency on external Rez installation.

## Phase 0 - Inventory + Contract Freeze
1. Freeze CLI/IO contracts for: config, context (rxt), build.rxt, variant.json, pip package layout.
2. Map all Python imports used at runtime (current Rust->Python entry points).
3. Define minimal Python surface: required modules, functions, and data exchanged with Rust.
4. Add snapshot tests for:
   - `build.rxt` schema
   - `context.rxt` schema
   - `variant.json` content

## Phase 1 - Config Fully in Rust
1. Implement `pkg-config` loader in Rust with Rez precedence rules and env overrides.
2. Provide `rezconfig.py` compatibility layer:
   - Option A (preferred): parse a restricted subset (assignments + dicts) in Rust.
   - Option B: execute `rezconfig.py` in embedded Python with a minimal stub module set.
3. Convert config to canonical `pkg-rs.toml` on first run.
4. Ensure `pkg rez config` matches Rez output fields.

## Phase 2 - Resolver + Context in Rust (No Python)
1. Replace Python solver usage with Rust resolver backend (Rez-compatible + native).
2. Implement full `.rxt` serialization in Rust:
   - fields/format matching `rez.resolved_context` output.
3. Implement graph output and diff in Rust.
4. Remove `rez.resolved_context` dependency.

## Phase 3 - Build Pipeline in Rust (No Rez Python)
1. Complete variant-aware build contexts (already started).
2. Implement build.rxt/variant.json parity and hashed variant shortlinks.
3. Port Rez build systems (custom/make/cmake/cargo/python) as Rust plugins.
4. Implement local+central build process and release flow in Rust.
5. Remove `rez.build_*` and `rezplugins.build_system` usage.

## Phase 4 - Pip/UV in Rust
1. Implement pip/uv discovery and execution in Rust.
2. Parse metadata and map to Rez package layout in Rust.
3. Implement PEP440 -> Rez requirement conversion in Rust.
4. Remove `rez.pip` and `rez.vendor.distlib` usage.

## Phase 5 - CLI Parity in Rust
1. Port all remaining `rez-*` commands to Rust.
2. Remove `rez.cli` passthrough.
3. Provide completion and help parity.

## Phase 6 - Python Tree Reduction
1. Delete `python/rezgui` (GUI owned by Rust node editor).
2. Delete `python/rezplugins` (all build systems and config schemas in Rust).
3. Delete `python/rez` (Rez core), except any minimal `rezconfig.py` shim if required.
4. Keep only:
   - `python/pkg.pyi`
   - `python/pkg_bootstrap.py` (if needed)
   - optional `python/rezconfig.py` shim

## Phase 7 - Cleanup + Validation
1. Remove `ensure_rez_on_sys_path` requirement from Rust runtime.
2. Ensure `pkg` works in a clean machine with no external Python/Rez.
3. Add integration tests for build + pip + context.

## Deliverables
- Rust-only runtime for all core features.
- Minimal embedded Python for `package.py` and optional `rezconfig.py`.
- Reduced `python/` directory (no Rez source, no vendors).
- Documentation update describing supported compat layer.

## Immediate Next Actions
1. Implement Phase 1 config loader in Rust and add `rezconfig.py` shim mode.
2. Replace solver usage in `src/solver/mod.rs` with Rust backend by default.
3. Remove `rezgui` from quickstart bind list and delete `python/rezgui`.
