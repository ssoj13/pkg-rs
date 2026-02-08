# pkg-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Package manager for VFX/DCC applications: dependency resolution, environment setup, and application launch for Maya, Houdini, Nuke, and similar tools.

**Note:** The ELF/Mach-O patching used for relocatable bundles is based on [arwen](https://github.com/nichmor/arwen). That code was heavily reworked, reduced to the needs of this project, and renamed to **bin-patch**; it lives in `crates/bin-patch` and is only used here.

---

## Overview

- **Package definitions** — Python `package.py` files (Rez-style), executed via embedded Python
- **Dependency resolution** — SAT solver (PubGrub) or Rez backend; request e.g. `maya + redshift`, get a compatible set
- **Environment** — PATH, PYTHONPATH, license and plugin paths from package definitions, with token expansion
- **Launch** — `pkg env maya -- maya.exe` runs the app with the resolved environment
- **Single binary** — no system Python required to run; optional Rez backend when Rez is installed

## Quick start

```powershell
# List packages
pkg list
pkg info maya

# Environment and launch
pkg env maya
pkg env maya -- maya.exe

# Export env to script
pkg env maya -o env.ps1
```

Package roots come from Rez-style config (`rezconfig.py`, `REZ_PACKAGES_PATH`, `~/.rezconfig`). Fallback: `./repo` if no paths are set.

## Installation

```powershell
cargo install pkg-rs
```

From source:

```powershell
.\bootstrap.ps1 build
.\bootstrap.ps1 python -i   # build Python module
```

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

## Configuration

Rez-compatible layering: `rezconfig.py` → `REZ_CONFIG_FILE` → `~/.rezconfig` → env overrides (`REZ_*`, `REZ_*_JSON`). pkg-specific options under `plugins.pkg_rs.*` (e.g. `resolver_backend`: `"pkg"` or `"rez"`).

## Commands

| Command | Description |
|---------|-------------|
| `pkg list` | List packages (`-L` latest only) |
| `pkg info <pkg>` | Package details |
| `pkg env <pkg>` | Print or export environment; `-- cmd` runs command |
| `pkg graph <pkg>` | Dependency graph (DOT/Mermaid) |
| `pkg scan` | Rescan locations |
| `pkg shell` | Interactive shell |
| `pkg py` | Python REPL |

## Docs

- [USERGUIDE.md](USERGUIDE.md) — workflows and usage
- [AGENTS.md](AGENTS.md) — architecture and dataflow (for contributors)

## License

MIT
