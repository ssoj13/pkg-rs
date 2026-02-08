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
