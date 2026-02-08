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
| ISS-007 | Install pipeline | Copy payload into repo path and write package metadata (including variant ids). | Installed package is discoverable by `pkg list` and resolves in `pkg env`. |
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
