# PLAN: Python -> Rust Port (pkg-rs)

## Goal
Keep Rez parity in Rust while preserving Python where needed (build systems, pip tooling).
Minimize only the *embedded rez* Python stack when Rust parity makes it safe.

## Current State (Feb 8, 2026)
- `rezconfig.py` executed directly via embedded Python; no `rez` module required.
- Build pipeline, pip import, resolver (PubGrub) implemented in Rust.
- Optional Python rez-solver still available (but should be replaced).
- `python/` contains `rez`, `rezgui`, `rezplugins` (mostly unused or replaceable).

## Phase 1: Inventory + Removal Safety
1. Inventory Python runtime usage in Rust:
   - `src/loader.rs` -> `package.py` execution (required).
   - `src/config.rs` -> `rezconfig.py` execution (required).
   - `src/solver/mod.rs` -> optional Python rez-solver (to replace).
2. Identify unused Python packages:
   - `python/rezgui` (GUI already in Rust) -> remove.
   - `python/rezplugins` (build systems, release hooks) -> remove **only if** Rust plugin parity lands.
   - `python/rez` -> remove after rez-solver is ported.
3. Add CI check to ensure no accidental import of `rez` modules except for explicit optional solver mode.

## Phase 2: Rez Solver Port (Core Parity)
1. Implement Rez dependency solver semantics in Rust:
   - Request ordering, package filters, and constraints.
   - Variant resolution and platform/arch/os matching.
   - Conflict explanation parity (best-effort).
2. Add config switch to choose solver (`pkg` vs `rez`), default to Rust port.
3. Remove Python rez-solver path once Rust parity is acceptable.

## Phase 3: Rez Plugins Parity (Rust Modules)
1. Define Rust plugin traits:
   - `BuildSystem`, `BuildProcess`, `ReleaseHook`, `Shell`, `PackageFilter`, `PackageOrderer`.
2. Implement built-in modules:
   - Build systems: cargo (done), custom, make, cmake, python (optional).
   - Build process: local/central (done).
3. Config-driven enabling/disabling per plugin type.

## Phase 4: Package Definition Evolution
1. Keep `package.py` support for compatibility.
2. Introduce optional `package.toml` (Rust-native) with 1:1 field mapping.
3. Add converter: `package.py` -> normalized Rust struct -> toml emission.
4. Decide long-term DSL strategy after parity reached.

## Phase 5: Cleanup
1. Remove `python/rezgui`, `python/rezplugins`.
2. Remove `python/rez` once solver port is done.
3. Document the minimal embedded Python contract.

## Deliverables
- `Rust solver` with Rez-parity tests.
- `Rust plugin framework` with build systems as modules.
- Optional `package.toml` path.
- Clean `python/` tree with only minimal runtime support.
