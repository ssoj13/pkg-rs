# pkg-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Package manager for VFX/DCC pipelines: Rez-compatible CLI, dependency resolution, environment setup, build, pip import, and application launch. Single binary; optional embedded Python Rez for bind quickstart and package definitions.

**Note:** ELF/Mach-O patching for relocatable bundles is based on [arwen](https://github.com/nichmor/arwen). That code was reworked, reduced, and renamed to **bin-patch** (`crates/bin-patch`); it is only used in this project.

---

## Overview

- **Package definitions** — Python `package.py` (Rez-style), executed via embedded Python; `pre_commands`, `commands`, `post_commands` run at `pkg env` (rex-style); `pre_test_commands` and `tests` at `pkg test`.
- **Dependency resolution** — PubGrub (native) or Rez backend; config: `plugins.pkg_rs.resolver_backend`.
- **Environment** — Merge envs, token expansion, stamp (`PKG_*`), export or run command after `--`.
- **Build / pip** — `pkg build`, `pkg pip` (import pip package into repo).
- **All rez-style commands are native** — no passthrough to Python Rez CLI; unsupported flags return an error.

## Quick start

```powershell
# List and resolve
pkg list
pkg info maya
pkg env maya
pkg env maya -- maya.exe
pkg env maya -o env.ps1

# Bind (module registry: platform, arch, os, python, rez, …)
pkg rez bind --list
pkg rez bind platform
pkg rez bind --quickstart

# Test package
pkg rez test mypkg
```

Package roots: Rez config (`rezconfig.py`, `REZ_PACKAGES_PATH`, `~/.rezconfig`). Fallback: `./repo`.

---

## Installation

```powershell
cargo install pkg-rs
```

From source:

```powershell
.\bootstrap.ps1 build
.\bootstrap.ps1 python -i   # build Python module
```

---

## Bind modules

Bind turns system software into Rez packages. The set of bindable names is a **module registry** you can extend or trim.

- **Native (Rust):** `platform`, `arch`, `os` — detect version, write `package.py` into local/release repo.
- **Python:** `python`, `rez`, `rezgui`, `setuptools`, `pip` — call `rez.package_bind.bind_package(name)`.

**Add modules** (all use Python strategy): in your Rez config (e.g. `~/.rezconfig` or file from `REZ_CONFIG_FILE`):

```python
plugins = {
    "pkg_rs": {
        "bind_modules_extra": ["mymodule", "other"],
        "bind_modules_remove": ["pip"],
    }
}
```

**Remove modules:** list names in `bind_modules_remove`; they disappear from `--list` and cannot be bound.

- `pkg rez bind --list` — all registered modules with `[native]` / `[python]`.
- `pkg rez bind --search [pattern]` — filter by name.
- `pkg rez bind <name>` — bind one (from registry).
- `pkg rez bind --quickstart` — bind all from registry (skip already installed).

---

## Configuration

Rez-style layering: `rezconfig.py` → `REZ_CONFIG_FILE` → `~/.rezconfig` → env (`REZ_*`, `REZ_*_JSON`). pkg-rs options under **`plugins.pkg_rs`**:

| Key | Description |
|-----|-------------|
| `resolver_backend` | `"pkg"` (PubGrub) or `"rez"` |
| `bind_modules_extra` | List of bind module names (Python strategy) |
| `bind_modules_remove` | List of names to exclude from bind registry |

---

## Commands (summary)

| Command | Description |
|---------|-------------|
| `pkg list` | List packages (`-L` latest only) |
| `pkg info <pkg>` | Package details |
| `pkg env <pkg>` | Env (pre/commands/post run); `-o` export; `-- cmd` run |
| `pkg graph <pkg>` | Dependency graph (DOT/Mermaid) |
| `pkg scan` | Rescan locations |
| `pkg shell` | Interactive shell |
| `pkg rez bind` | Bind modules (--list, --search, &lt;name&gt;, --quickstart) |
| `pkg rez context` | .rxt: --print-request, --print-resolve, --format, --which, … |
| `pkg rez status` | Rez version, active context, visible suites |
| `pkg rez suite` | Suites: --list, --create, DIR |
| `pkg rez test <pkg>` | Run pre_test_commands + tests section |
| `pkg build` | Build package (current dir package.py) |
| `pkg pip` | Import pip package into repo |
| `pkg rez search` | Search packages (patterns, tags, --latest) |
| `pkg rez view <pkg>` | Package details |
| `pkg rez depends` | Dependency graph (list/dot/mermaid) |
| `pkg rez config` | Config paths and values |
| `pkg gui` | Node editor GUI (graph, solve, export env) |
| `pkg version` | Version and build info |

Full status and per-command behaviour: [PLAN.md](PLAN.md) (§2).

---

## package.py

```python
from pkg import Package, Env, Evar, App

def get_package():
    pkg = Package("houdini", "21.0.440")
    env = Env("default")
    env.add(Evar("HFS", "/opt/hfs21.0.440", "set"))
    env.add(Evar("PATH", "{HFS}/bin", "append"))
    pkg.add_env(env)
    pkg.add_app(App("houdini").with_path("{HFS}/bin/houdini"))
    pkg.add_req("redshift@>=3.5")
    return pkg
```

Optional: `pre_commands`, `commands`, `post_commands` (run at `pkg env`); `pre_test_commands` and `tests` (run at `pkg test`).

---

## Python API

```python
from pkg import Package, Env, Evar, App, Storage, Solver

storage = Storage.scan()
solver = Solver(storage.packages)
solution = solver.solve("maya-2026.1.0")

p = Package("mytool", "1.0.0")
p.add_req("maya@>=2024")
env = Env("default")
env.add(Evar("PATH", "{ROOT}/bin", "append"))
p.add_env(env)
```

---

## Docs

| Doc | Description |
|-----|-------------|
| [PARITY.md](PARITY.md) | Rez parity: what's done, what's left, rough estimate (~85% for typical use) |
| [PLAN.md](PLAN.md) | Plan, status of all commands (§2), done/todo (§3–4), Python dir (§5) |
| [AGENTS.md](AGENTS.md) | Architecture, dataflow, codepaths (contributors) |
| [md/USERGUIDE.md](md/USERGUIDE.md) | Workflows and usage |

---

## License

MIT
