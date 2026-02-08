# Rez Parity Audit Report (Full Stack)

Date: 2026-02-08

## Scope
- Full Rez parity assessment for CLI, config, package format, repository backends, resolver/context, build, pip, shells, caching.
- Evidence is from local source files in `D:/_pkg-rs` and reference Rez in `D:/_pkg-rs/_ref/rez`.

## Evidence Sources (local)
- Rez CLI entry points: `D:/_pkg-rs/_ref/rez/src/rez/cli/_entry_points.py:65-316`
- Rez config precedence: `D:/_pkg-rs/_ref/rez/src/rez/rezconfig.py:5-35`
- Rez package schema: `D:/_pkg-rs/_ref/rez/src/rez/package_resources.py:130-210`, `D:/_pkg-rs/_ref/rez/src/rez/package_serialise.py:18-110`
- Rez build system/process: `D:/_pkg-rs/_ref/rez/src/rez/build_system.py:13-205`, `D:/_pkg-rs/_ref/rez/src/rez/build_process.py:26-167`
- Rez pip behavior: `D:/_pkg-rs/_ref/rez/src/rez/pip.py:34-220`
- Rez resolved context: `D:/_pkg-rs/_ref/rez/src/rez/resolved_context.py:55-200`
- Rez repository plugins: `D:/_pkg-rs/_ref/rez/src/rez/package_repository.py:16-220`
- pkg-rs CLI commands: `D:/_pkg-rs/src/pkg/cli.rs:56-260`, `D:/_pkg-rs/src/pkg/commands/mod.rs:3-23`
- pkg-rs config loader + overrides: `D:/_pkg-rs/src/config.rs:1-260`, `D:/_pkg-rs/src/py.rs:1-80`, `D:/_pkg-rs/python/rez/rezconfig.py`
- pkg-rs package model: `D:/_pkg-rs/src/package.rs:220-420`
- pkg-rs loader command extraction: `D:/_pkg-rs/src/loader.rs:238-430`
- pkg-rs storage scan: `D:/_pkg-rs/src/storage.rs:9-205`
- pkg-rs solver: `D:/_pkg-rs/src/solver/mod.rs:1-260`
- pkg-rs build pipeline: `D:/_pkg-rs/src/build.rs:22-260`
- pkg-rs pip pipeline: `D:/_pkg-rs/src/pip.rs:3-233`

## TODO/FIXME Scan Summary
- pkg-rs: no TODO/FIXME/HACK markers in Rust sources; upstream TODOs exist in vendored Rez Python (`python/rez` + `python/rezplugins`) and should be treated as parity notes rather than new defects.

## Progress Update
- Switched config loading to Rez Python algorithm (`rez.config`) with embedded rezconfig.py defaults, REZ_CONFIG_FILE list, ~/.rezconfig, and REZ_* / REZ_*_JSON overrides. Evidence: `D:/_pkg-rs/src/config.rs:1-260`, `D:/_pkg-rs/src/py.rs:1-80`, `D:/_pkg-rs/python/rez/rezconfig.py`.
- Updated storage/build/pip to consume Rez config keys (`packages_path`, `local_packages_path`, `release_packages_path`, `plugins.pkg_rs.pip_install_remaps`). Evidence: `D:/_pkg-rs/src/storage.rs:479-514`, `D:/_pkg-rs/src/build.rs:462-530`, `D:/_pkg-rs/src/pip.rs:820-980`.
- Added resolver backend selection via `plugins.pkg_rs.resolver_backend` (default: pkg backend). Evidence: `D:/_pkg-rs/src/config.rs:45-90`, `D:/_pkg-rs/src/solver/mod.rs:20-120`, `D:/_pkg-rs/src/package.rs:1303-1350`.
- Implemented Rez resolver backend using embedded Python `rez.resolved_context`, with temporary config swap and variant-safe name extraction. Evidence: `D:/_pkg-rs/src/solver/mod.rs:85-210`, `D:/_pkg-rs/python/rez/resolved_context.py:165-340`, `D:/_pkg-rs/python/rez/packages.py:355-369`.
- Extended Package schema with Rez fields (commands, tests/help, release metadata, relocatable/cachable, plugin flags, extras) and updated serialization; loader captures command sources from globals or function source. Evidence: `D:/_pkg-rs/src/package.rs:220-1170`, `D:/_pkg-rs/src/loader.rs:238-430`.
- Added dataflow/codepath diagrams for current vs target command execution pipeline. Evidence: `D:/_pkg-rs/AGENTS.md:240-320`, `D:/_pkg-rs/DIAGRAMS.md:30-80`, `D:/_pkg-rs/diagram.md:40-95`.
- Vendored `rezplugins` into the embedded Python tree to satisfy `rez.config` imports. Evidence: `D:/_pkg-rs/python/rezplugins/__init__.py`.
- Embedded runtime now fails fast if `rezplugins` is missing. Evidence: `D:/_pkg-rs/src/py.rs:50-75`.
- CMake build system now reads Rez plugin config (`plugins.build_system.cmake.*`), injects default `cmake_args` + `CMAKE_MODULE_PATH`, and supports generator/env overrides; emits Windows SDK warnings when LIB is missing. Evidence: `D:/_pkg-rs/src/build/systems/cmake.rs:1-320`.
- Added MSVC environment bootstrap (vcv-rs port) to populate PATH/INCLUDE/LIB/LIBPATH when missing; controlled via `plugins.pkg_rs.msvc_auto` and `PKG_MSVC_*` overrides; build now fails fast if VS/SDK/UCRT are missing. Evidence: `D:/_pkg-rs/src/build/msvc.rs:1-494`, `D:/_pkg-rs/src/build.rs:262-310`.
- CLI build args now allow hyphen-leading values for `--build-args` and `--child-build-args`. Evidence: `D:/_pkg-rs/src/pkg/cli.rs:135-155`.
- Added rez subcommand group inside the single `pkg` binary; `pkg rez env/build/pip` route to existing handlers, remaining rez commands are stubs. Evidence: `D:/_pkg-rs/src/pkg/cli.rs:90-520`, `D:/_pkg-rs/src/pkg/main.rs:150-330`.
- Implemented `pkg rez config` parity (search/source list, key lookup, yaml/json output) using rez config loader. Evidence: `D:/_pkg-rs/src/pkg/commands/rez_config.rs:1-80`, `D:/_pkg-rs/src/pkg/main.rs:52-70`.
- Added embedded Rez CLI pass-through for `rez bind/context/status/suite`, capturing Python stdout/stderr and emitting via Rust. Evidence: `D:/_pkg-rs/src/pkg/commands/rez_passthrough.rs:1-120`, `D:/_pkg-rs/src/pkg/main.rs:60-92`.
- Vendored `rezgui` into the embedded Python tree to support `rez-bind rezgui` (quickstart). Evidence: `D:/_pkg-rs/python/rezgui/`.
- Bound quickstart packages `platform`, `arch`, `os`, `python`, `rez`, `rezgui`, `setuptools`, `pip` into `C:\Users\joss1\packages` using `pkg rez bind`. Evidence: `C:/Users/joss1/packages/*`.
- Added an idempotent native quickstart path for `pkg rez bind --quickstart` that skips already-installed packages and avoids FileExists errors. Evidence: `D:/_pkg-rs/src/pkg/commands/rez_bind.rs:1-230`.
- Removed legacy TOML config schema/defaults (`D:/_pkg-rs/src/config_schema.rs`, `D:/_pkg-rs/config/default.toml`) to avoid divergence.
- Remaining config parity gaps: plugin settings validation, typed schema enforcement, and wiring non-repo config keys into runtime behavior.

## Test Results (C:\temp)

- `pkg ls -L` with `-r C:\temp\pkg-repo` succeeded after adding `rezplugins`. Evidence: `C:/temp/pkg-tests/pkg_ls2.out`.
- `pkg info blender` and `pkg info jangafx` show registered apps and env counts. Evidence: `C:/temp/pkg-tests/pkg_info_blender.out`, `C:/temp/pkg-tests/pkg_info_jangafx.out`.
- `pkg env blender --format json` returns BLENDER_ROOT and PATH entries. Evidence: `C:/temp/pkg-tests/pkg_env_blender.out`.
- `pkg build` for `cargo_hello` succeeded and installed to `C:\temp\pkg-repo\cargo_hello\0.1.0`. Evidence: `C:/temp/pkg-tests/pkg_build_cargo2.out`, `C:/temp/pkg-tests/pkg_build_cargo2.err`.
- `pkg build` for `cmake_hello` succeeded after MSVC env bootstrap; generator args accepted via `--build-args "-G Ninja"`. Evidence: `C:/temp/pkg-tests/pkg_build_cmake9.out`, `C:/temp/pkg-tests/pkg_build_cmake9.err`.
- `pkg pip appdirs` and `pkg pip PySide6` succeeded with rez-style package output. Evidence: `C:/temp/pkg-tests/pkg_pip_appdirs.out`, `C:/temp/pkg-tests/pkg_pip_pyside6.out`.

## Findings (Gaps vs Rez)

### 1) CLI parity is missing many Rez commands
- Severity: High
- Evidence: Rez exposes 30+ CLI entry points including `rez-env`, `rez-build`, `rez-pip`, `rez-suite`, `rez-context`, `rez-bind`, `rez-pkg-cache`, `rez-yaml2py`, `rez-bundle`, `rez-benchmark`, `rez-pkg-ignore`, `rez-mv`, `rez-rm` in `D:/_pkg-rs/_ref/rez/src/rez/cli/_entry_points.py:65-316`. pkg-rs defines only a subset of commands in `D:/_pkg-rs/src/pkg/cli.rs:56-260` and `D:/_pkg-rs/src/pkg/commands/mod.rs:3-23`.
- Impact: `pkg rez env/build/pip` are now available, but most Rez commands remain unimplemented stubs.
- Recommendation: Add all Rez commands and alias behavior with consistent flags and outputs within the single `pkg` binary.

### 2) Config precedence now Rez-native, runtime wiring still incomplete
- Severity: Medium
- Evidence: Rez config is loaded via `rez.config` with rezconfig.py defaults, REZ_CONFIG_FILE, ~/.rezconfig, and env overrides (`D:/_pkg-rs/src/config.rs:1-260`, `D:/_pkg-rs/python/rez/rezconfig.py`). 
- Impact: Core config semantics now match Rez; remaining gap is wiring all config keys into runtime behavior (resolver filters/orderers, shells, caching, memcache, etc.).
- Recommendation: Connect rez config keys to resolver/context/build/pip/shell pipelines and eliminate legacy pkg-rs config assumptions.

### 3) Package schema parity is partial (structure added, runtime still missing)
- Severity: Medium
- Evidence: Rez schema includes command fields and metadata (`D:/_pkg-rs/_ref/rez/src/rez/package_resources.py:130-210`). pkg-rs now exposes these fields and serializes them (`D:/_pkg-rs/src/package.rs:220-1170`) and extracts command sources in the loader (`D:/_pkg-rs/src/loader.rs:238-430`).
- Impact: Schema-level parity is closer, but runtime behavior still diverges because command execution, tests runner, and help/tests usage are not wired into context/build/test flows.
- Recommendation: Implement command execution in resolved contexts, add package test runner parity, and ensure help/tests/revision metadata are surfaced in CLI/reporting.

### 4) Repository backend plugin system is missing
- Severity: High
- Evidence: Rez defines repository plugins and a repository interface (`D:/_pkg-rs/_ref/rez/src/rez/package_repository.py:16-220`). pkg-rs uses a single filesystem scanning `Storage` implementation (`D:/_pkg-rs/src/storage.rs:9-205`).
- Impact: No parity for memory repositories, repository-specific payload rules, or plugin-based discovery.
- Recommendation: Introduce repository trait + plugin registry and backends.

### 5) Resolver/context features are missing or simplified
- Severity: High
- Evidence: Rez `ResolvedContext` supports package filters, orderers, timestamps, patch locks, suite visibility, and context serialization (`D:/_pkg-rs/_ref/rez/src/rez/resolved_context.py:55-200`). pkg-rs now exposes a Rez solver backend, but still lacks full context/runtime layering and rez-style execution (`D:/_pkg-rs/src/solver/mod.rs:85-210`, `D:/_pkg-rs/src/build.rs:554-615`).
- Impact: Resulting environments differ from Rez in resolution behavior and serialization.
- Recommendation: Add a resolver layer above PubGrub and support Rez-compatible context logic and `.rxt` serialization.

### 6) Build parity gaps beyond current pipeline
- Severity: High
- Evidence: Rez has plugin-based build systems and build processes with local/central flows (`D:/_pkg-rs/_ref/rez/src/rez/build_system.py:13-205`, `D:/_pkg-rs/_ref/rez/src/rez/build_process.py:26-167`). pkg-rs uses a single build pipeline without plugins (`D:/_pkg-rs/src/build.rs:22-260`).
- Impact: Missing build process parity and extensibility.
- Recommendation: Add build system plugin trait, build process abstraction, and build.rxt/build-env script parity.

### 7) Pip parity gaps remain
- Severity: High
- Evidence: Rez pip discovers rezified python/pip and enforces pip>=19 with min_deps/no_deps behavior (`D:/_pkg-rs/_ref/rez/src/rez/pip.py:34-220`). pkg-rs pip uses local python discovery and a simplified requirements conversion (`D:/_pkg-rs/src/pip.rs:3-233`).
- Impact: Dependency resolution and payload layout differ from Rez pip.
- Recommendation: Port Rez pip discovery order, PEP440 conversion, and RECORD remap behavior.

### 8) Shell plugins and env output parity is missing
- Severity: Medium
- Evidence: Rez uses shell plugins for command and env output (`D:/_pkg-rs/_ref/rez/src/rez/resolved_context.py:126-135`). pkg-rs currently emits env output in a simplified format via CLI (`D:/_pkg-rs/src/pkg/cli.rs:93-119`).
- Impact: Shell-specific behavior and scripting differs from Rez.
- Recommendation: Implement shell plugin system and per-shell output parity.

### 9) Caching/memcache parity is incomplete
- Severity: Medium
- Evidence: Rez supports resolve caching, package file caching, listdir caching, and memcached settings (`D:/_pkg-rs/_ref/rez/src/rez/rezconfig.py:141-183`). pkg-rs reads these keys via rezconfig (`D:/_pkg-rs/src/config.rs:1-260`), but the runtime cache/memcache behavior is not implemented.
- Impact: Performance and behavioral differences under load.
- Recommendation: Implement cache/memcache behavior in resolver, storage, and build pipelines to honor the config.

### 10) CMake builds still depend on external toolchain installation
- Severity: Medium
- Evidence: MSVC auto-env bootstrap now succeeds when Visual Studio + SDK are installed (`C:/temp/pkg-tests/pkg_build_cmake9.err` shows detected VS/SDK versions) and fails fast when VS/SDK/UCRT are missing (`D:/_pkg-rs/src/build/msvc.rs:106-140`).
- Impact: Builds now stop immediately on empty machines instead of failing later in compiler tests.
- Recommendation: Keep the MSVC bootstrap, document required toolchain installs, and add clearer diagnostics when VS/SDK are missing.

## Recommendation
Proceed with the parity implementation plan in `D:/_pkg-rs/plan9.md` and track completion in `D:/_pkg-rs/TODO.md`.
