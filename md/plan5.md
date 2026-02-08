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
Step 2: wire command execution and tests into runtime; validate schema parity with real packages.