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
