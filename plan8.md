# Plan 8 - Rez Solver Backend Validation + Parity Tests

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
