# Plan 6 - Rez Parity Implementation (Command Execution Focus)

Date: 2026-02-07

## Goal
Implement Rez-compatible command execution and testing flow on top of the extended package schema.

## Steps
1. Add ResolvedContext layer (or equivalent) to execute pre/commands/post via rex-like engine.
2. Wire command execution into `pkg env` and app launch paths.
3. Implement pre_test_commands + tests execution with report output.
4. Add fixtures to compare against Rez behavior for commands/tests.
5. Update diagrams and report as behavior lands.

## Current Focus
Step 1: design ResolvedContext and rex command execution flow.