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
