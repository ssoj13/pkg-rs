# Progress

## Summary
- Core package system is live in Rust with Rez-style versioning, requirements, and name parsing (hyphen-digit rule).
- Rez-like build pipeline and pip import are implemented in Rust.
- Rez config (`rezconfig.py`) is parsed via embedded Python; config access is Rust.
- Rez CLI parity is partial: `rez env/build/pip` are native; several `rez *` are stub/passthrough.

## Rez Command Parity Table

| Rez Command | Status | Notes |
|---|---|---|
| `rez env` | Native | Implemented via `pkg env`/`pkg rez env` (Rust). |
| `rez build` | Native | Implemented via `pkg build`/`pkg rez build` (Rust). |
| `rez pip` | Native | Implemented via `pkg pip`/`pkg rez pip` (Rust). |
| `rez bind` | Partial | Rust quickstart/native subset + fallback to Python. |
| `rez config` | Partial | Rust config reader and output; supports `--search-list`, `--source-list`, `--json`, `field`. |
| `rez context` | Partial | Rust subset (`--req/--res/--pg/--wg/--format/--which`), falls back for advanced flags. |
| `rez status` | Partial | Rust subset; falls back for advanced flags (e.g. `--tools`). |
| `rez suite` | Partial | Rust subset (`--list`, `--create`); falls back for advanced flags. |
| `rez cp` | Native | File-based copy with variants, rename/reversion, shallow, overwrite. |
| `rez depends` | Native | List/dot/mermaid dependency output. |
| `rez diff` | Native | Filesystem diff with external difftool fallback. |
| `rez gui` | Stub | Passthrough placeholder. |
| `rez help` | Stub | Passthrough placeholder. |
| `rez interpret` | Stub | Passthrough placeholder. |
| `rez memcache` | Native | Config-only status output (no memcache backend yet). |
| `rez pkg-cache` | Native | Manage package metadata cache (list/stats/clear). |
| `rez plugins` | Native | Lists packages that plug into a given host package. |
| `rez python` | Stub | Passthrough placeholder. |
| `rez release` | Stub | Passthrough placeholder. |
| `rez search` | Native | Search packages with tags/latest/json. |
| `rez selftest` | Stub | Passthrough placeholder. |
| `rez test` | Stub | Passthrough placeholder. |
| `rez view` | Native | View package metadata in JSON/text. |
| `rez yaml2py` | Stub | Passthrough placeholder. |
| `rez bundle` | Stub | Passthrough placeholder. |
| `rez benchmark` | Stub | Passthrough placeholder. |
| `rez pkg-ignore` | Native | `.ignore<version>` handling for filesystem repo. |
| `rez mv` | Native | Copy + ignore source (filesystem repo). |
| `rez rm` | Native | Remove package/family or ignored-since. |
| `rez _rez-complete` | Stub | Passthrough placeholder. |
| `rez _rez_fwd` | Stub | Passthrough placeholder. |

## Task: Command Unification (Requested)

We currently have mixed legacy `pkg-rs` commands and the new Rez-compatible command structure. We need to move legacy `pkg-rs` commands (and their implementations) out of the primary CLI surface (do not delete, just relocate), and switch to a Rez-style command system under `pkg`:

- Target CLI structure: `pkg env`, `pkg build`, `pkg pip`, `pkg config`, `pkg context`, etc.
- Implement *all* Rez commands natively in Rust (no Python execution for Rez logic).
- Python is only for: reading `rezconfig.py`, executing `package.py`, and providing the embedded REPL/interpreter.

### Immediate next steps
1. Inventory and move legacy commands/handlers out of the main CLI (keep code accessible but not active).
2. Replace `pkg rez <command>` wrapper with direct `pkg <command>` mapping and full option parity.
3. Implement missing Rez commands in Rust; remove passthrough stubs.
4. Expand tests for command parity and expected outputs.
