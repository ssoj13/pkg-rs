# Rez Parity Audit Report (Full Stack)

Date: 2026-02-07

## Scope
- Full Rez parity assessment for CLI, config, package format, repository backends, resolver/context, build, pip, shells, caching.
- Evidence is from local source files in `D:/_pkg-rs` and reference Rez in `D:/_pkg-rs/_ref/rez`.

## Evidence Sources (local)
- Rez CLI entry points: `D:/_pkg-rs/_ref/rez/src/rez/cli/_entry_points.py:65-316`
- Rez config precedence: `D:/_pkg-rs/_ref/rez/src/rez/rezconfig.py:5-35`
- Rez package schema: `D:/_pkg-rs/_ref/rez/src/rez/package_serialise.py:18-110`
- Rez build system/process: `D:/_pkg-rs/_ref/rez/src/rez/build_system.py:13-205`, `D:/_pkg-rs/_ref/rez/src/rez/build_process.py:26-167`
- Rez pip behavior: `D:/_pkg-rs/_ref/rez/src/rez/pip.py:34-220`
- Rez resolved context: `D:/_pkg-rs/_ref/rez/src/rez/resolved_context.py:55-200`
- Rez repository plugins: `D:/_pkg-rs/_ref/rez/src/rez/package_repository.py:16-220`
- pkg-rs CLI commands: `D:/_pkg-rs/src/pkg/cli.rs:56-260`, `D:/_pkg-rs/src/pkg/commands/mod.rs:3-23`
- pkg-rs config loader: `D:/_pkg-rs/src/config.rs:68-177`
- pkg-rs package model: `D:/_pkg-rs/src/package.rs:161-259`
- pkg-rs storage scan: `D:/_pkg-rs/src/storage.rs:9-205`
- pkg-rs solver: `D:/_pkg-rs/src/solver/mod.rs:1-215`
- pkg-rs build pipeline: `D:/_pkg-rs/src/build.rs:22-260`
- pkg-rs pip pipeline: `D:/_pkg-rs/src/pip.rs:3-233`

## TODO/FIXME Scan Summary
- pkg-rs: no TODO/FIXME/HACK markers detected outside `_ref`.

## Findings (Gaps vs Rez)

### 1) CLI parity is missing many Rez commands
- Severity: High
- Evidence: Rez exposes 30+ CLI entry points including `rez-env`, `rez-build`, `rez-pip`, `rez-suite`, `rez-context`, `rez-bind`, `rez-pkg-cache`, `rez-yaml2py`, `rez-bundle`, `rez-benchmark`, `rez-pkg-ignore`, `rez-mv`, `rez-rm` in `D:/_pkg-rs/_ref/rez/src/rez/cli/_entry_points.py:65-316`. pkg-rs defines only a subset of commands in `D:/_pkg-rs/src/pkg/cli.rs:56-260` and `D:/_pkg-rs/src/pkg/commands/mod.rs:3-23`.
- Impact: Users cannot achieve Rez-equivalent workflows in pkg-rs CLI.
- Recommendation: Add all Rez commands and alias behavior with consistent flags and outputs.

### 2) Config precedence and overrides are incomplete
- Severity: High
- Evidence: Rez config layering includes config files list, home config, env overrides, JSON env overrides, and package config section override for build/release (`D:/_pkg-rs/_ref/rez/src/rez/rezconfig.py:8-26`). pkg-rs currently supports only `--cfg`, `PKG_RS_CONFIG`, binary directory `pkg-rs.toml`, and `~/.pkg-rs/pkg-rs.toml`, and writes a default config if missing (`D:/_pkg-rs/src/config.rs:68-177`).
- Impact: Parity-breaking config behavior, especially for env overrides and build/release overrides.
- Recommendation: Implement Rez-style precedence using TOML and add env override rules.

### 3) Package schema parity is partial
- Severity: High
- Evidence: Rez schema includes `tools`, `commands`, `pre_commands`, `post_commands`, `pre_test_commands`, `tests`, `help`, `config`, `timestamp`, `release_message`, and `changelog` (`D:/_pkg-rs/_ref/rez/src/rez/package_serialise.py:18-110`). pkg-rs `Package` lacks most of these fields (`D:/_pkg-rs/src/package.rs:161-259`).
- Impact: Package definitions cannot be ported without loss and runtime behavior diverges.
- Recommendation: Extend pkg-rs package model and loader to include missing Rez fields.

### 4) Repository backend plugin system is missing
- Severity: High
- Evidence: Rez defines repository plugins and a repository interface (`D:/_pkg-rs/_ref/rez/src/rez/package_repository.py:16-220`). pkg-rs uses a single filesystem scanning `Storage` implementation (`D:/_pkg-rs/src/storage.rs:9-205`).
- Impact: No parity for memory repositories, repository-specific payload rules, or plugin-based discovery.
- Recommendation: Introduce repository trait + plugin registry and backends.

### 5) Resolver/context features are missing or simplified
- Severity: High
- Evidence: Rez `ResolvedContext` supports package filters, orderers, timestamps, patch locks, suite visibility, and context serialization (`D:/_pkg-rs/_ref/rez/src/rez/resolved_context.py:55-200`). pkg-rs `Solver` is a direct PubGrub resolver without these layers (`D:/_pkg-rs/src/solver/mod.rs:1-215`).
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

### 9) Caching/memcache parity is missing
- Severity: Medium
- Evidence: Rez supports resolve caching, package file caching, listdir caching, and memcached settings (`D:/_pkg-rs/_ref/rez/src/rez/rezconfig.py:141-183`). pkg-rs has no equivalent cache configuration in config schema (`D:/_pkg-rs/src/config.rs:29-202`).
- Impact: Performance and behavioral differences under load.
- Recommendation: Add cache configuration and memcache integration.

## Recommendation
Proceed with the parity implementation plan in `D:/_pkg-rs/plan3.md` and track completion in `D:/_pkg-rs/TODO.md`.