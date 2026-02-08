# TODO - Rez Parity Roadmap for pkg-rs

Date: 2026-02-08

## Goal
Full behavioral parity with Rez CLI and runtime, implemented in Rust with embedded Python, plus modular crates for build systems, resolvers, repository backends, and plugins.

## Non-Negotiables
- Keep embedded Python as the execution/runtime surface for `package.py` and Rex-like command logic.
- Use Rez config (rezconfig.py + REZ_CONFIG_FILE + ~/.rezconfig) as the source of truth.
- Preserve current pkg-rs improvements, but make Rez parity the baseline.
- Modularize so build systems, resolvers, repository backends, and shells are pluggable.

## Rez Feature Inventory (Must Match)
- CLI commands: `rez`, `rezolve`, `_rez-complete`, `_rez_fwd`, `rez-bind`, `rez-build`, `rez-config`, `rez-context`, `rez-cp`, `rez-depends`, `rez-diff`, `rez-env`, `rez-gui`, `rez-help`, `rez-interpret`, `rez-memcache`, `rez-pip`, `rez-pkg-cache`, `rez-plugins`, `rez-python`, `rez-release`, `rez-search`, `rez-selftest`, `rez-status`, `rez-suite`, `rez-test`, `rez-view`, `rez-yaml2py`, `rez-bundle`, `rez-benchmark`, `rez-pkg-ignore`, `rez-mv`, `rez-rm`.
- Config precedence: base defaults, config files list, home config, env overrides (`REZ_*` and `REZ_*_JSON`), package `config` section overrides, and plugin section exception rules.
- Package definition schema: `name`, `version`, `description`, `authors`, `tools`, `requires`, `build_requires`, `private_build_requires`, `variants`, `hashed_variants`, `relocatable`, `cachable`, `commands`, `pre_commands`, `post_commands`, `pre_build_commands`, `pre_test_commands`, `help`, `tests`, `timestamp`, `revision`, `release_message`, `changelog`, `config`, plus custom keys.
- Build process: build-system plugin detection, build process plugin, per-variant builds, build.rxt serialization, build-env scripts, `parse_build_args.py` exports, standard `REZ_BUILD_*` env vars, local vs central build flow, release flow and VCS integration.
- Resolver and context: request parsing, variant selection, package filters, package orderers, timestamp locks, patch locks, implicit packages, context serialization, suite visibility, tool visibility, graph output, rex execution.
- Repository backends: filesystem repo as default, memory repo for tests, cacheable repositories, package payload install APIs, variant URI routing.
- Pip: rezified python/pip discovery order, PEP440 conversion, min_deps/no_deps logic, distlib-based metadata, entry points, payload remap rules, rez-style package metadata.
- Caching and memcache: resolve caching, package file caching, listdir caching, resource cache size, memcached settings and invalidation.
- Shell plugins: bash/zsh/csh/cmd/pwsh with correct env formatting and alias semantics.

## Proposed Crate Layout (Modular)
- `pkg-core`: Package model, env, evar, app, dep spec, serialization.
- `pkg-config`: Rez config loader (rez.config) + plugin overrides and validation.
- `pkg-repo`: Repository trait, filesystem backend, memory backend, package cache.
- `pkg-resolver`: Resolver trait, Rez-compatible solver, PubGrub solver.
- `pkg-context`: Resolved context, serialization, graph output, suite handling.
- `pkg-build`: Build process, build env, build.rxt, install logic.
- `pkg-build-systems`: Build system plugin trait + implementations (custom, make, cmake, cargo).
- `pkg-pip`: rez-pip parity pipeline.
- `pkg-shell`: Shell plugin system and env emitters.
- `pkg-cli`: CLI command routing, help, completion.
- `pkg-plugins`: Plugin registry for build systems, repo backends, shells, VCS, release hooks.

## Integration Plan (High-Level)
1. Config parity
2. Package schema parity
3. Repository backend parity
4. Resolver parity
5. Context and suite parity
6. Build parity
7. Pip parity
8. CLI parity
9. Shell parity
10. Caching and memcache parity

## Detailed Workstreams

### Config Parity
- Load Rez config via embedded `rez.config` with rezconfig.py defaults, REZ_CONFIG_FILE list, ~/.rezconfig, and REZ_* / REZ_*_JSON overrides. (Done)
- Add package `config` section override for build/release contexts. (Done)
- Vendor `rezplugins` into the embedded Python tree so `rez.config` imports succeed. (Done)
- Validate config values against Rez schema (including plugin settings). (Pending)
- Wire remaining Rez config keys into runtime behavior (resolver filters/orderers, shells, caching, memcache). (Pending)

### CLI Parity
- Add all Rez command equivalents under `pkg rez <cmd>` within the single `pkg` binary.
- Implement rez stubs for unported commands, and route `pkg rez env/build/pip` to existing handlers.
- Implement `pkg rez config` output parity (search/source lists + key lookup + yaml/json output). (Done)
- Implement pass-through execution for rez CLI commands via embedded Python where Rust parity is not ready (bind/context/status/suite). (Done)
- Add `rezgui` to embedded Python tree for quickstart bind. (Done)
- Implement idempotent `rez bind --quickstart` in Rust (skip existing packages, reuse embedded bind modules). (Done)
- Implement alias behavior (`rezolve` style) and forwarders.
- Implement `pkg config`, `pkg context`, `pkg suite`, `pkg status` equivalents.

### Package Schema Parity
- Extend `Package` struct to include missing Rez fields. (Done)
- Support `package.yaml` and `package.py` I/O parity. (Pending)
- Implement help/tests/config sections and command wrappers. (Partial: schema + loader capture done; runtime execution pending)

### Repository Parity
- Implement repository trait with filesystem backend and memory backend.
- Implement package payload paths, variant URIs, and repository config settings.
- Implement package cache and `pkg-cache` operations.

### Resolver Parity
- Implement Rez-compatible solver and make it selectable. (Done: embedded Python rez resolver)
- Preserve PubGrub as optional resolver backend.
- Add package filters, orderers, timestamp locks, patch locks, implicit packages.

### Context and Suite Parity
- Implement `.rxt` serialization compatible with Rez.
- Implement suite semantics, tool visibility, and suite visibility rules.
- Implement graph outputs compatible with `rez-context` and `rez-env`.

### Build Parity
- Build process plugin with local and central flows.
- Build system plugin detection and child build system handling.
- Allow `--build-args` and `--child-build-args` values starting with `-` (CLI fix). (Done)
- Add toolchain/generator config (CMake generator, toolchain file) and Windows SDK detection guidance. (Partial: generator/env support + SDK warning + MSVC env bootstrap)
- Build env scripts and `parse_build_args.py` export behavior.
- Standard `REZ_BUILD_*` env vars and hashed variant shortlinks.

### Pip Parity
- Rezified python/pip discovery order.
- Full PEP440 to rez requirement conversion.
- Distlib metadata parsing and RECORD-based copy with remap rules.
- Entry point wrapper generation and metadata attributes.

### Shell Parity
- Implement shell plugins with correct quoting and alias semantics.
- Add `pkg env --shell` behavior consistent with Rez.

### Caching and Memcache Parity
- Implement resolve caching and memcached integration.
- Implement package file caching and listdir caching.

## Tests and Validation
- Create parity tests for each CLI command.
- Add integration tests for `pkg build` (cargo + cmake) and `pkg pip` (appdirs + PySide6) using a temp repo.
- Create cross-check tests against Rez reference output for:
  - config resolution
  - context serialization
  - build env scripts
  - pip metadata mapping
  - resolver results for known package graphs

## Deliverables
- Parity audit report with evidence and gaps.
- Modular crates with clean APIs.
- Migration guide from Rez to pkg-rs.
- Comprehensive test suite and sample repositories.
