# Plan 1: Rez Parity Implementation (build + pip)

## Goals
- Achieve functional parity with Rez for build and pip workflows.
- Preserve pkg-rs architecture while matching Rez behaviors where needed.

## Steps
1. Extend package schema and loader
   - Add `variants`, `hashed_variants`, `private_build_requires`, `pre_build_commands`, `requires_rez_version`.
   - Change `build_command` type to support `False | str | list`.
   - Update Python bindings and `python/pkg.pyi`.

2. Implement variant-aware build context
   - Compute variant requires and subpaths (hashed and non-hashed).
   - Resolve build context using `build_requires + private_build_requires + variant reqs`.
   - Align REZ_BUILD_* env vars with Rez semantics.

3. Build process & install pipeline
   - Add BuildProcess (local) and BuildSystem trait.
   - Implement per-variant build dirs and build.rxt snapshot.
   - Install payload, variant metadata (`variant.json`), and extra files.
   - Implement variant shortlinks when `hashed_variants` is true.

4. Build systems parity
   - Custom build: placeholder expansion, list commands, `build_command=False`, `parse_build_args.py` env export.
   - CMake build: generator settings, module path, REZ_BUILD_DOXYFILE, and build/install phases.
   - Make build: thread count, install target, child build args.

5. CLI parity
   - Add `--process`, `--fail-graph`, `--build-args`/`--child-build-args` parsing with `--`.
   - `--view-pre` to emit preprocessed package definition (Rez-like output).

6. Pip parity
   - Implement rezified python/pip discovery (resolver-based), enforce pip>=19.
   - Add install modes (min_deps/no_deps) and `--use-pep517` default.
   - Port distlib file mapping + `pip_install_remaps`.
   - Port PEP440 -> Rez requirement conversion.
   - Add hashed variants and pip metadata to generated package.

7. Tests
   - Build: custom, make, cmake with variants and install.
   - Pip: pure python + platform wheel, dependency resolution, entry points.

## Status
- 0/7 steps completed.
# Plan 2 - Rez Parity Completion (build + pip)

1. Build: emit Rez-compatible `build.rxt` (ResolvedContext schema) and generate a `build-env` forwarding script that spawns a build shell from `build.rxt`.
2. Build: add build process abstraction (local/central), expose `--process` in CLI, set `REZ_BUILD_TYPE` and `REZ_IN_REZ_RELEASE` accordingly.
3. Build: align `pre_build_commands` context with Rez (VariantBinding-like `this`, RO_AttrDictWrapper-like `build`), and add `parse_build_args.py` support with `__PARSE_ARG_*` exports.
4. Build: optional hashed variant shortlinks (`_v`) with config flag and resolution logic.
5. Pip: resolve python/pip via Storage/Solver (rezified python/pip first) and implement min_deps/no_deps behavior.
6. Pip: port full PEP440 -> Rez requirement conversion (including `!=` unions and wildcard rules).
7. Pip: implement distlib/RECORD mapping with configurable remaps (`pip_install_remaps`) for payload layout.
8. Pip: persist pip metadata fields (`pip_name`, `from_pip`, `is_pure_python`, `help`, `authors`, `tools`) in Package and emit them in generated `package.py`.

## Test Plan
- Build: use example package with variants and pre_build_commands; verify `build.rxt` load + `build-env` behavior matches Rez.
- Build: test `--process local|central` flags and REZ_* variables.
- Pip: import a package with markers and `!=` constraints; verify requirements match Rez output.
- Pip: verify RECORD mapping and entry points in repo layout.
# Plan 3 - Rez Parity Implementation

Date: 2026-02-07

## Goal
Implement full Rez parity in pkg-rs with modular crates and embedded Python runtime.

## Steps
1. Define crate layout and move existing modules into crate boundaries without breaking current CLI.
2. Implement Rez-style config precedence and env overrides in `pkg-config`.
3. Implement repository trait and filesystem backend parity in `pkg-repo`.
4. Implement resolver layer and ResolvedContext serialization with `.rxt` parity.
5. Implement build process and build system plugin registry in `pkg-build` and `pkg-build-systems`.
6. Implement rez-pip parity in `pkg-pip` including PEP440 conversion and distlib-style payload mapping.
7. Implement shell plugin system and per-shell env output.
8. Implement all Rez CLI commands and aliases in `pkg-cli`.
9. Add parity test suite and reference fixtures.

## Current Status
- Step 2 (Config parity) completed: Rez defaults in TOML, full PKG_/REZ_ env overrides + JSON overrides, package config overrides, repo alias sync.
# Plan 4 - Rez Parity Implementation (Config Complete)

Date: 2026-02-07

## Goal
Full Rez parity in pkg-rs with modular crates, embedded Python runtime, and TOML config.

## Steps
1. Config parity: Rez defaults in TOML, full env overrides, package config override, repo alias mapping. (Done)
2. Package schema parity: add missing Rez fields (commands, pre/post commands, tests, timestamp, revision, changelog, relocatable/cachable, etc.).
3. Repository parity: repository trait + filesystem/memory backends, cacheable repos, variant URIs.
4. Resolver/context parity: Rez-compatible resolver layer, filters/orderers, timestamp/patch locks, suite visibility, .rxt serialization.
5. Build parity: build process plugins, build system plugins, build.rxt/build-env scripts, local/central flows.
6. Pip parity: rezified python discovery order, distlib metadata, RECORD remap rules, entry points.
7. Shell parity: shell plugin system and per-shell env output.
8. CLI parity: all Rez commands and aliases.
9. Caching/memcache parity.
10. Tests + parity fixtures.

## Current Focus
Step 2: extend Package schema and loader/serializer to match Rez fields.
# Plan 5 - Rez Parity Implementation (Package Schema Extended)

Date: 2026-02-07

## Goal
Full Rez parity in pkg-rs with modular crates, embedded Python runtime, and TOML config.

## Steps
1. Config parity (Done)
2. Package schema parity (In Progress)
   - Add missing fields + serialization (Done)
   - Capture commands from package.py globals (Done)
   - Wire command execution + tests into runtime (Pending)
3. Repository parity: repo trait + filesystem/memory backends, cacheable repos, variant URIs (Pending)
4. Resolver/context parity: filters/orderers, timestamp/patch locks, suite visibility, .rxt serialization (Pending)
5. Build parity: build process plugins, build system plugins, build.rxt/build-env scripts, local/central flows (Pending)
6. Pip parity: rezified python discovery order, distlib metadata, RECORD remap rules, entry points (Pending)
7. Shell parity: shell plugin system and per-shell env output (Pending)
8. CLI parity: full Rez command surface + aliases (Pending)
9. Caching/memcache parity (Pending)
10. Tests + parity fixtures (Pending)

## Current Focus
Step 2: wire command execution and tests into runtime; validate schema parity with real packages.# Plan 6 - Rez Parity Implementation (Command Execution Focus)

Date: 2026-02-07

## Goal
Implement Rez-compatible command execution and testing flow on top of the extended package schema.

## Steps
1. Add ResolvedContext layer (or equivalent) to execute pre/commands/post via rex-like engine.
2. Wire command execution into `pkg env` and app launch paths.
3. Implement pre_test_commands + tests execution with report output.
4. Add fixtures to compare against Rez behavior for commands/tests.
5. Update diagrams and report as behavior lands.

## Current Focus
Step 1: design ResolvedContext and rex command execution flow.# Plan 7 - Rez Config Wiring + Parity Tests

Date: 2026-02-08

## Goal
Complete Rez-native config wiring and validate parity with targeted build/pip/solver tests.

## Steps
1. Audit remaining runtime paths for config usage and wire missing Rez keys (filters/orderers/caching/memcache/shells).
2. Implement actual Rez solver backend (embedded Python resolver) and expose selection via `plugins.pkg_rs.resolver_backend`.
3. Add config validation for `plugins.pkg_rs` and surface schema errors clearly.
4. Create test packages under `C:\temp` (Blender5, JangaFX) and run `pkg build`/`pkg env` to verify install targets.
5. Run `pkg pip` parity tests for `appdirs` and `PySide6` and verify payload layout + requirements.
6. Update diagrams/report and document any remaining parity gaps.

## Current Focus
Step 1: config wiring audit and missing key integration.# Plan 8 - Rez Solver Backend Validation + Parity Tests

Date: 2026-02-08

## Goal
Validate the Rez solver backend, wire remaining Rez config keys, and run parity tests (build/pip) using C:\temp packages.

## Steps
1. Wire remaining Rez config keys into runtime behavior (filters/orderers/caching/memcache/shells).
2. Validate Rez backend end-to-end (resolve via embedded Python, ensure variant names map to package names).
3. Create test packages under `C:\temp` (Blender5, JangaFX, simple cmake/cargo) and run `pkg build`/`pkg env`.
4. Run `pkg pip` parity tests for `appdirs` and `PySide6`; verify payload layout + requirements.
5. Update diagrams/report/TODO with test outcomes and remaining gaps.

## Current Focus
Step 1: remaining config wiring (filters/orderers/caching/memcache/shells).
# Plan 9 - Build CLI Fixes + CMake Toolchain + Integration Tests

Date: 2026-02-08

## Goal
Stabilize build CLI argument handling, make CMake toolchain setup configurable, and codify build/pip integration tests.

## Steps
1. Allow hyphen-leading values for `build_args`/`child_build_args` in CLI (Clap `allow_hyphen_values`) and update help text. (Done)
2. Add a startup check that verifies `python/rezplugins` exists and emits a clear error if missing. (Done)
3. Add CMake toolchain/generator config (CMAKE_GENERATOR, toolchain file, optional vcvarsall/vsdevcmd bootstrap) and validate SDK presence with actionable error messages. (Partial: generator/env support + SDK warning + MSVC env bootstrap)
4. Add integration tests for cargo/cmake build and pip (appdirs, PySide6) using a temp repo. (Done: manual runs in C:\temp)
5. Update diagrams/report/TODO after implementation and tests. (In progress)

## Current Focus
Step 5: finalize docs/diagrams and remaining toolchain bootstrap work.
# Plan 10 - Rez CLI Parity (Single Binary + Alias Map)

Date: 2026-02-08

## Goal
Provide Rez-compatible commands as subcommands of a single `pkg` binary (no separate rez-* binaries), then progressively implement parity.

## Steps
1. Add `rez <cmd>` subcommands within `pkg` (single binary).
2. Introduce shared `Args` structs for env/build/pip/list/info to avoid duplicated flag definitions.
3. Map implemented Rez commands to existing handlers (`cmd_env`, `cmd_build`, `cmd_pip`).
4. For remaining Rez commands, add explicit stubs that fail fast with a clear parity TODO.
5. Add tests:
   - `pkg rez env` / `pkg rez build` / `pkg rez pip` parse and run (same as base commands)
6. Update docs/diagrams/report/TODO with mapping table and remaining gaps.

## Notes
- Single binary only: no argv0 multicall or separate executables.
- `rez`/`rezolve` are not separate commands; use `pkg` with subcommands.

## Approval Gate
Proceed to implementation after review of CLI mapping and test approach.
﻿# Plan 11 - Full Rust Port + Python Minimization

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
# Integration Plan: Rez Features in pkg-rs

**Objectives**
- Add a build pipeline (build + install + release) to pkg-rs.
- Add a pip-to-pkg import workflow (rez-pip equivalent).
- Introduce plugin points for build systems, release hooks, and VCS.
- Define a compatibility strategy between Rez package definitions and pkg-rs package.py.

**Constraints / Assumptions**
- pkg-rs package definitions are Python files that instantiate `Package/Env/App` objects.
- No build process exists in pkg-rs today; build hooks exist in template only.
- Prefer a local build process first; remote build can be deferred.

**Integration Options**
1. External Rez bridge
Pros: minimal development; reuse full Rez build/release/pip immediately.
Cons: requires Rez + system Python; splits behavior between tools; harder to make deterministic.
2. Native build pipeline in Rust
Pros: single binary; consistent UX; better performance; no external dependencies.
Cons: more implementation work; must re-create Rez behaviors.
3. Hybrid (embedded Python build runner)
Pros: reuse Rez-like build scripts without full Rez install; keeps single binary.
Cons: embedded Python needs careful packaging; still some Rez behaviors to reimplement.

**Phased Plan**
1. Phase 0: Compatibility mapping
Deliverable: schema matrix and conversion rules.
- Map Rez package fields to pkg-rs equivalents (requires, variants, commands, build metadata).
- Decide whether to support a Rez-compatible parser or a converter.
- Define build metadata fields for pkg-rs: `build_system`, `build_command`, `build_requires`, `build_directory`.
2. Phase 1: Build core (local only)
Deliverable: `pkg build` command with custom build system.
- Add `pkg build` CLI with flags: `--install`, `--prefix`, `--clean`, `--variants`, `--scripts`, `--build-system`, `--build-args`.
- Implement `BuildSystem` trait and `custom` build system (runs `build_command`).
- Implement `BuildProcess` local: per-variant build directory, build env creation, build logs.
- Define build environment variables (PKG_BUILD_* analogs to REZ_BUILD_*).
3. Phase 2: Install + release
Deliverable: payload installation and package definition updates.
- Add local/release package paths config.
- Implement install to repo (payload + package.py update).
- Add pre_install test hooks (optional gate).
- Add release metadata and tagging stubs.
4. Phase 3: Pip import
Deliverable: `pkg pip` command.
- Implement find-pip logic (choose python/pip version; fallback to embedded).
- `pip install --target` into temp dir.
- Parse metadata, convert requirements (PEP440 -> pkg-rs ranges), copy files into `python/` + `bin/`.
- Generate pkg-rs package definition with commands for PYTHONPATH/PATH.
5. Phase 4: Plugins and build systems
Deliverable: plugin registry and first-party plugins.
- Build system plugins: `cmake`, `make`.
- Release hooks and VCS plugins.
- Shell integration and completions parity with Rez where useful.
6. Phase 5: Advanced Rez features (selective)
Deliverable: feature parity where it adds value.
- Suites, context bundles, ephemerals.
- Package orderers and caching improvements.
- GUI parity decisions.

**Validation**
- Build and install a simple CMake package.
- Pip-import a pure Python wheel and a platform wheel.
- Ensure `pkg env` reproduces build/install environments deterministically.
- Add regression tests around build and pip import flows.

## Feature Mapping Table
| Rez Feature | Evidence | pkg-rs Status | Gap / Work | Integration Option | Priority | Effort |
| --- | --- | --- | --- | --- | --- | --- |
| Package definitions (package.py, Package/Env/App) | docs: package_definition | Supported | Align field semantics and metadata | Native | P0 | M |
| Variants and variant-specific requirements | docs: variants | Partial | Extend variant resolution, overrides | Native | P0 | M |
| Commands / environment setup | docs: package_commands | Partial | Map Rez command semantics to pkg-rs env builder | Native | P0 | M |
| Context resolution (dependency solver) | docs: context | Supported | Close semantic gaps and edge cases | Native | P0 | M |
| `rez-env` style environment activation | cli: env | Supported | Add build/runtime split and context export | Native | P0 | M |
| Package repositories (filesystem) | plugins: package_repository | Supported | Add repository config parity and versioning rules | Native | P0 | M |
| Package repositories (memory/virtual) | plugins: package_repository | Missing | Add in-memory repo for tests/dev | Native | P2 | S |
| Package orderers | plugins: package_orderers | Missing | Implement ordering hooks | Native | P2 | M |
| Caching / memcache | docs: caching, cli: memcache | Missing | Add resolve cache + invalidation | Native | P1 | M |
| Package search | cli: search | Missing | Add query CLI and filters | Native | P1 | M |
| Package view / info | cli: view | Partial | Extend to match rez view fields | Native | P2 | S |
| Depends / graph | cli: depends | Partial | Add reverse-deps and variants | Native | P1 | M |
| Diff contexts | cli: diff | Missing | Add context diff reporting | Native | P2 | M |
| Repo maintenance (cp/mv/rm) | cli: cp/mv/rm | Missing | Add repo file operations | Native | P2 | S |
| Package ignore | cli: pkg-ignore | Missing | Add ignore rules and CLI | Native | P2 | S |
| Package cache inspection | cli: pkg-cache | Missing | Add cache inspection tools | Native | P2 | S |
| Build system plugins (custom/make/cmake) | plugins: build_system | Missing | Implement BuildSystem trait + cmake/make | Native | P0 | L |
| Build process (local) | plugins: build_process | Missing | Local build directories, logs, env vars | Native | P0 | L |
| Build process (remote) | plugins: build_process | Missing | Remote execution and artifact fetch | Defer | P2 | L |
| Release pipeline | cli: release | Missing | Add release CLI and metadata | Native | P1 | M |
| Release hooks (email/command/amqp) | plugins: release_hook | Missing | Add hook system | Native | P2 | M |
| Release VCS integration (git/hg/svn) | plugins: release_vcs | Missing | Add VCS abstraction and git first | Native | P1 | M |
| Pip import (`rez-pip`) | cli: pip + utils | Missing | Implement `pkg pip` flow | Native | P0 | L |
| Python/Rez API | docs: api | Missing | Expose Rust API and optional Python bindings | Hybrid | P2 | M |
| Shell plugins (bash/zsh/csh/cmd/pwsh) | plugins: shell | Partial | Add per-shell setup and hooks | Native | P1 | M |
| CLI completions | cli: complete | Supported | Expand subcommands | Native | P2 | S |
| GUI | cli: gui | Partial | Decide to keep or replace | Defer | P3 | M |
| Suites | docs: suites | Missing | Implement suite definitions and activation | Defer | P3 | M |
| Context bundles | docs: context_bundles | Missing | Add bundle build/export | Defer | P3 | L |
| Ephemerals | docs: ephemerals | Missing | Add ephemeral package generation | Defer | P3 | M |
| `rez-yaml2py` | cli: yaml2py | Missing | Add conversion tool | Defer | P3 | S |
| `rez-bind` / `rez-forward` | cli: bind/forward | Missing | Add wrappers if needed | Defer | P3 | S |
| `rez-interpret` / `rez-context` | cli: interpret/context | Missing | Add context export + script runner | Defer | P2 | M |
| Testing / selftest / benchmark | cli: test/selftest/benchmark | Partial | Add build/pip integration tests | Native | P1 | M |

## Release 1 Scope (P0/P1 Only)
**In scope**
- Build core: local build process + build systems (custom, make, cmake).
- Build CLI and build environment variables.
- Install to repo + release metadata stubs.
- Pip import (`pkg pip`) with requirements conversion and file layout.
- Cache + search + depends improvements.
- Shell integration parity for cmd/pwsh and common POSIX shells.
- Tests covering build and pip import.

**Explicitly out of scope**
- Remote build process.
- Suites, context bundles, ephemerals.
- GUI parity, yaml2py, bind/forward.

## Task Breakdown
| Task | Description | Depends | Effort | Priority |
| --- | --- | --- | --- | --- |
| Build metadata schema | Add build fields to pkg definitions (build_system, build_command, build_requires, build_dir) | Compatibility mapping | M | P0 |
| Build CLI | Add `pkg build` command + flags, wire to build pipeline | Build metadata schema | M | P0 |
| Build system trait | Implement `BuildSystem` trait + registry | Build metadata schema | M | P0 |
| Build system: custom | Execute user-defined build command | Build system trait | M | P0 |
| Build system: make | Implement make plugin + default args | Build system trait | M | P0 |
| Build system: cmake | Implement cmake plugin + toolchain config | Build system trait | L | P0 |
| Local build process | Per-variant build dirs, env export, logs | Build system trait | L | P0 |
| Install pipeline | Copy payload to repo, write package.py metadata | Local build process | M | P0 |
| Release metadata | Add release info and version tagging (no hooks yet) | Install pipeline | M | P1 |
| Pip import core | `pkg pip` flow: pip discovery, temp target, file mapping | Build metadata schema | L | P0 |
| Pip requirements | Convert PEP440 -> pkg-rs ranges, system reqs | Pip import core | M | P0 |
| Pip commands | Generate PYTHONPATH/PATH commands from entry points | Pip import core | M | P0 |
| Cache layer | Resolve cache + invalidation policy | Context resolution | M | P1 |
| Search command | Add `pkg search` with filters | Cache layer | M | P1 |
| Depends enhancements | Reverse-deps + variants | Context resolution | M | P1 |
| Shell integration | Implement per-shell setup (cmd/pwsh/bash/zsh) | Env activation | M | P1 |
| Test suite | Build + pip import regression tests | Build CLI, Pip import | M | P1 |

## MVP Definition (Rez-Parity)
**Goal**
- Implement the minimal build + pip import pipeline with behavior intentionally matched to Rez.

**Rez Parity Rules**
- Build environment variables mirror Rez conventions (REZ_BUILD_* style names, variant-aware).
- Build runs per-variant with isolated build directories.
- Build uses build system plugins with `custom`, `make`, and `cmake` parity.
- Install writes payload into a repo package path and updates package metadata.
- Pip import uses `pip install --target` into temp, converts requirements, maps entry points into commands.
- Pip import creates a package with hashed variants or equivalent deterministic variant id.

**MVP Scope**
- `pkg build` local-only with custom/make/cmake.
- `pkg install` (or `pkg build --install`) to repo path.
- `pkg pip` to import a wheel/sdist into a repo package.
- Build scripts mode (`--scripts`) to reproduce build environment without running the build.
- Basic cache + search + depends parity required to validate build/pip flows.
- cmd/pwsh + bash/zsh shell hooks needed for Windows/Linux parity.

**MVP Execution Order**
1. Build metadata schema and build CLI.
2. Build system trait + custom build system.
3. Local build process + build env variables.
4. Install pipeline (repo write + metadata).
5. Add make/cmake build systems.
6. Build scripts mode (env script generation).
7. Pip import core (pip discovery, temp target, file mapping).
8. Pip requirement conversion + entry point commands.
9. Cache/search/depends enhancements.
10. Shell integration for cmd/pwsh/bash/zsh.
11. Regression tests for build and pip import.

## Issue List (MVP)
| ID | Title | Description | Acceptance |
| --- | --- | --- | --- |
| ISS-001 | Build metadata schema | Add build fields to package model and python loader for build_system, build_command, build_requires, build_directory, build_args. | `package.py` parser exposes these fields and they appear in the in-memory Package model. |
| ISS-002 | Build CLI | Add `pkg build` command with flags for install, prefix, clean, variants, build-system, build-args, verbose, quiet. | CLI runs build pipeline and supports variant filtering and install mode. |
| ISS-003 | Build system trait | Implement `BuildSystem` trait and registry for plugin selection by name. | Custom build system can be invoked via CLI flag or package metadata. |
| ISS-004 | Local build process | Implement local build process with per-variant build dirs, env export, logs, and `build.rxt` snapshot. | Each variant produces an isolated build directory and a saved build context file. |
| ISS-005 | Build env parity | Export Rez-compatible build env vars (REZ_BUILD_*) with variant-aware values. | Build scripts can read the same env vars as Rez. |
| ISS-006 | Build scripts mode | Implement `--scripts` to generate build environment scripts without running the build system. | Generated scripts reproduce the build env and can be executed manually. |
| ISS-007 | Install pipeline | Copy payload into repo path and write package metadata (including variant ids). | Installed package is discoverable by `pkg search` and resolves in `pkg env`. |
| ISS-008 | Build system: make | Implement make plugin that honors REZ_BUILD_THREAD_COUNT and install path. | `pkg build` succeeds for a simple Makefile package. |
| ISS-009 | Build system: cmake | Implement cmake plugin with configure, build, install phases. | `pkg build` succeeds for a simple CMake package. |
| ISS-010 | Pip discovery | Implement pip discovery order with python/pip packages or system fallback. | `pkg pip` can run without manual pip path configuration. |
| ISS-011 | Pip install core | Implement `pip install --target` into temp and collect dist metadata. | A wheel or sdist installs into temp and produces a staging tree. |
| ISS-012 | Pip requirement conversion | Convert PEP440 requirements to pkg-rs ranges and emit system requirements. | Package requirements match pip metadata within accepted lossiness. |
| ISS-013 | Pip commands | Convert entry points into commands/tools and add PYTHONPATH/PATH env. | Installed pip package is runnable from `pkg env`. |
| ISS-014 | Cache layer | Implement resolve cache with invalidation on repo changes. | Repeated resolves are faster and consistent. |
| ISS-015 | Search command | Add `pkg search` with name/version/tag filters. | Search returns results from local and configured repos. |
| ISS-016 | Depends enhancements | Add reverse dependencies and variant-aware dependency reports. | `pkg depends` shows forward and reverse deps with variants. |
| ISS-017 | Shell integration | Implement env activation for cmd/pwsh/bash/zsh. | `pkg env` works consistently on Windows and Linux. |
| ISS-018 | Build + pip tests | Add regression tests for build and pip import pipelines. | CI runs build/pip tests and they pass on at least one platform. |

## Technical Design (Rez-Parity)
### Build Pipeline
The build pipeline mirrors Rez and is built around a `BuildProcess` that iterates variants and invokes a `BuildSystem` plugin per variant.
1. Parse `package.py` and resolve variants.
2. Resolve a build context that includes `build_requires` and variant requirements.
3. Compute build root from `build_directory` config, default `build`, relative to package source.
4. For each variant, compute a variant subpath and create an isolated build dir under the build root.
5. Export build environment variables for the variant, then run `pre_build` or Rez-style `pre_build_commands` if present.
6. If `--scripts` is set, emit build environment scripts and skip execution.
7. Invoke the selected build system plugin.
8. Run `post_build` or Rez-style `post_build_commands`.
9. If install mode is enabled, run install stage and write package payload to the repo path.
10. Save a build context snapshot `build.rxt` in the build root for debugging.

### Build Environment Variables
These variables are exported to match Rez behavior and naming.
| Name | Value |
| --- | --- |
| REZ_BUILD_ENV | Always `1` during build. |
| REZ_BUILD_PATH | Absolute build root path. |
| REZ_BUILD_THREAD_COUNT | Thread count from config, default uses physical cores. |
| REZ_BUILD_VARIANT_INDEX | Variant index or 0 when not variantized. |
| REZ_BUILD_VARIANT_REQUIRES | Space-delimited variant requirement list. |
| REZ_BUILD_VARIANT_SUBPATH | Variant subpath relative to build root. |
| REZ_BUILD_PROJECT_VERSION | Package version. |
| REZ_BUILD_PROJECT_NAME | Package name. |
| REZ_BUILD_PROJECT_DESCRIPTION | Package description string. |
| REZ_BUILD_PROJECT_FILE | Absolute path to `package.py`. |
| REZ_BUILD_SOURCE_PATH | Absolute path to package source directory. |
| REZ_BUILD_REQUIRES | Space-delimited resolved requirements used for build. |
| REZ_BUILD_REQUIRES_UNVERSIONED | Space-delimited requirement names only. |
| REZ_BUILD_TYPE | `local` for MVP. |
| REZ_BUILD_INSTALL | `1` when install is enabled, else `0`. |
| REZ_BUILD_INSTALL_PATH | Absolute install path when install is enabled. |

### Build System Plugins
`BuildSystem` defines `configure`, `build`, `install`, and `clean` phases and receives the build context and env.
- `custom` runs the `build_command` as-is in the build dir.
- `make` runs `make -j$REZ_BUILD_THREAD_COUNT` and installs into `REZ_BUILD_INSTALL_PATH`.
- `cmake` runs configure with `-S` and `-B`, builds, then `cmake --install`.

### Install Pipeline
Install writes payload into a repo path structured as `{repo}/{name}/{version}/` with variant subpaths for hashed variants.
- Payload layout mirrors Rez for Python packages: `python/`, `bin/`, `lib/` where applicable.
- Metadata includes variant id hash derived from variant requirements and build requirements.
- Install is idempotent when target exists and `--clean` is not set.

### Pip Import Pipeline
`pkg pip` mirrors `rez-pip` behavior with a deterministic conversion step.
1. Discover pip using python/pip packages if present, otherwise fall back to system Python.
2. Validate pip version against `pip>=19` equivalent.
3. Run `pip install --target <temp>` with `--use-pep517` unless overridden.
4. Read dist metadata and RECORD to map installed files into package layout.
5. Convert PEP440 requirements into pkg-rs ranges and emit system requirements for platform, arch, and python.
6. Translate entry points to `commands` and `tools` and add PYTHONPATH/PATH env.
7. Write package.py and install into repo path.

### CLI and Config Surface
- `pkg build` mirrors Rez `rez-build` for local builds with `--install`, `--prefix`, `--clean`, `--variants`, `--build-system`, `--build-args`, and `--scripts`.
- `pkg pip` mirrors `rez-pip` and supports `--python-version`, `--install` or `--release`, `--prefix`, and `--extra`.
- Config defaults align with Rez: `build_directory = "build"`, `build_thread_count = physical_cores`, `pip_extra_args = []`.
# PLAN — План, статус команд и парность

Единый документ: текущее состояние, статус всех команд, что сделано, что в работе, что прибить. Детали: [md/PLAN.md](md/PLAN.md), [TODO.md](TODO.md), [PARITY.md](PARITY.md).

---

## 1. Текущее состояние (кратко)

- **pkg-rs**: package.py, резолвер (PubGrub/Rez), env + **pre/commands/post** (rex) при `pkg env`, **pre_test + tests** при `pkg test`, build, pip, **все rez-команды нативно** (bind — модульный регистр, extra/remove в конфиге). Неподдерживаемые опции → ошибка.
- **bin-patch** (`crates/bin-patch`): ELF/Mach-O для релоцируемых бандлов.
- **Windows**: санитизация путей python/pip после pip-импорта.

---

## 2. Статус команд (сводка)

**Легенда:** **Native** — реализация в Rust (или Rust + точечный вызов Python API). **Internal** — служебная команда.

### 2.1 Ядро

| Команда | Статус | Описание |
|---------|--------|----------|
| **env** | Native | Резолв, merge env, pre/commands/post (rex), stamp, expand; вывод или запуск после `--`. |
| **build** | Native | package.py, варианты, build context, pre_build_commands, build system (custom/make/cmake/cargo/python), установка. |
| **build-env** | Internal | Окружение сборки из build.rxt. |
| **pip** | Native | Поиск python/pip, pip install --target, метаданные, копирование в репо, package.py. |

### 2.2 Конфиг и контекст

| Команда | Статус | Описание |
|---------|--------|----------|
| **config** | Native | Чтение rezconfig, пути, поля, JSON. |
| **context** | Native | .rxt (REZ_RXT_FILE), --print-request/resolve, --format, --which и др.; неизвестное → ошибка. |
| **status** | Native | Без аргументов: версия, контекст, suites; с аргументами → ошибка. |
| **suite** | Native | --list, --create, DIR; неизвестное → ошибка. |

### 2.3 Bind

| Команда | Статус | Описание |
|---------|--------|----------|
| **bind** | Native | Регистр: Native (platform, arch, os, python, rez, setuptools, pip) + config extra. Конфиг: `bind_modules_extra` / `bind_modules_remove`. Clap: -l, -s, --quickstart, -r, -i, &lt;name&gt;. |

### 2.4 Поиск, граф, репозиторий

| Команда | Статус |
|---------|--------|
| **search, view, depends, diff** | Native |
| **cp, mv, rm, release, pkg-ignore, pkg-cache** | Native |

### 2.5 Тесты и прочее

| Команда | Статус |
|---------|--------|
| **test** | Native (pre_test rex, tests, --inplace) |
| **interpret** | Native |
| **plugins, memcache** | Native |
| **bundle, benchmark, yaml2py** | Native |
| **python, shell, gui, help, version, completions, selftest** | Native |

**Итог:** все команды Native или Internal; passthrough удалён.

---

## 3. Сделано (отмечено ✅)

| # | Задача | Отметка |
|---|--------|--------|
| 1 | Выполнение pre/commands/post при `pkg env` (rex) | ✅ |
| 2 | Тесты пакетов: pre_test_commands (rex) + секция tests, .rxt inplace | ✅ |
| 3 | Passthrough → native (bind, context, status, suite) | ✅ |
| 4 | Bind: модульный регистр, extra/remove в конфиге | ✅ |

---

## 4. В работе / надо сделать

| # | Задача | Приоритет | Пометка |
|---|--------|-----------|--------|
| 5 | Пути python/pip на Windows | по багам | Уже санитизация; при багах — не писать абсолютные пути в конфиг/репо. |
| 6 | Документация | высокий | ✅ USERGUIDE/AGENTS обновлены; примеры конфига (bind, resolver, filter/orderers); rex, test, shell, suite, caches. |
| 7 | Интеграционные тесты | высокий | ✅ rex (pre_commands invoke), test section (pre_test_commands loaded). |
| 8 | Filters/orderers в резолвере | средний | ✅ Уже подключены (backend=pkg + plugins); документировано. |
| 9 | Shell plugins, suite visibility | средний | ✅ Форматы env и правила suite описаны в USERGUIDE/AGENTS. |
| 10 | Кэши (resolve, memcache, package) | низкий | ✅ Package cache реализован; resolve/memcache задокументированы в AGENTS/USERGUIDE. |

---

## 5. Python каталог — когда нужен

**Встроенные bind-модули** (platform, arch, os, python, rez, setuptools, pip) реализованы в Rust; для них **python/ не нужен**.

**python/ обязателен только если** в конфиге задан `bind_modules_extra`: тогда при bind такого имени вызывается `rez.package_bind` (Python). В остальных случаях bind не трогает Python.

**Loader и pkg python:** вызов `ensure_rez_on_sys_path` сделан мягким (ошибка игнорируется). Без каталога python/ загрузка package.py и `pkg python` работают; если в package.py есть `import rez`, пользователь может добавить путь к rez в PYTHONPATH сам.

| Что | Когда нужен python/ |
|-----|----------------------|
| Загрузка package.py, `pkg python` | Нет (rez в path опционально). |
| `pkg bind` встроенные модули | Нет. |
| `pkg bind <имя из bind_modules_extra>` | Да (нужны python/rez/, python/rezplugins/). |
| CMake build system (Rez-шаблоны) | Если есть python/rezplugins/build_system/cmake_files. |

**Вывод: каталог python/ можно не класть** при сборке/распространении, если не используете bind_modules_extra и не нужны Rez CMake-модули. Для bind extra по-прежнему нужны `python/rez/` и `python/rezplugins/`.

---

## 6. Порядок выполнения

1. ~~п.1–4~~ — сделано.
2. ~~Консолидация~~ — PLAN + STATUS в один файл, STATUS.md удалён, ссылки обновлены. ✅
3. ~~Python: удалить `python/rez/tests/`~~ — удалено (тесты Rez не используются из pkg-rs). ✅
4. **Дальше:** п.6 (документация), п.7 (интеграционные тесты), п.8–10.

---

## 7. Ссылки

- [PARITY.md](PARITY.md) — парность с Rez, оценка (~85%).
- [md/PORT_TO_RUST.md](md/PORT_TO_RUST.md) — полный обзор границ Python и план портирования на Rust (config, solver, bind, pip, rex, loader).
- [md/PLAN.md](md/PLAN.md) — интеграционный план (build, pip, плагины).
- [TODO.md](TODO.md) — Rez parity roadmap.
- [AGENTS.md](AGENTS.md) — архитектура, потоки данных.
