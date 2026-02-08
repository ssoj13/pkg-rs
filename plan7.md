# Plan 7 - Rez Config Wiring + Parity Tests

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
Step 1: config wiring audit and missing key integration.