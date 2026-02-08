# Plan 10 - Rez CLI Parity (Single Binary + Alias Map)

Date: 2026-02-08

## Goal
Provide Rez-compatible commands as subcommands of a single `pkg` binary (no separate rez-* binaries), then progressively implement parity.

## Steps
1. Add `rez <cmd>` subcommands within `pkg` (single binary).
2. Introduce shared `Args` structs for env/build/pip/list/info to avoid duplicated flag definitions.
3. Map implemented Rez commands to existing handlers (`cmd_env`, `cmd_build`, `cmd_pip`).
4. For remaining Rez commands, add explicit stubs that fail fast with a clear parity TODO.
5. Add tests:
   - `pkg rez env` / `pkg rez build` / `pkg rez pip` parse and run (same as base commands)
6. Update docs/diagrams/report/TODO with mapping table and remaining gaps.

## Notes
- Single binary only: no argv0 multicall or separate executables.
- `rez`/`rezolve` are not separate commands; use `pkg` with subcommands.

## Approval Gate
Proceed to implementation after review of CLI mapping and test approach.
