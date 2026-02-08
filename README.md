# pkg-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)

![pkg-rs](docs/test.png)

Package manager for VFX/DCC applications. Manages environment configuration for Maya, Houdini, Nuke, and other pipeline software.

See [USERGUIDE.md](USERGUIDE.md) for detailed workflows.

## Overview

Manages multiple versions of DCC applications and plugins with automatic dependency resolution:

- **Dependency resolution** — request `maya + redshift + studio_tools`, solver finds compatible versions. Conflicts reported with clear diagnostics
- **Environment configuration** — PATH, PYTHONPATH, license servers, plugin paths configured automatically
- **Application launch** — `pkg env maya -- maya.exe` starts application with correct environment

## Features

- **Embedded Python** — package definitions are Python files; no Python installation required
- **SAT-based solver** — finds compatible versions automatically with clear conflict diagnostics
- **Token expansion** — variables like `{MAYA_ROOT}/bin` expand correctly
- **Fast** — parallel scanning, JSON cache, millisecond rescan times
- **CLI + Python API** — terminal usage or pipeline integration
- **Cross-platform** — Windows and Linux

## Comparison with rez

[rez](https://github.com/AcademySoftwareFoundation/rez) is the industry standard. Key differences:

| | pkg-rs | rez |
|---|--------|-----|
| Install | single binary | Python + pip + system packages |
| Scan 200 packages | 30ms warm | seconds |
| Solve time | ~90μs | milliseconds |
| Python for packages | embedded | system Python required |
| Windows | native | functional but complex setup |

pkg-rs targets smaller teams and simpler deployments. For large studios with existing rez infrastructure, migration may not be warranted.

## Installation

```powershell
cargo install pkg-rs
```

Single binary, no dependencies.

### From source

```powershell
# Build CLI
.\bootstrap.ps1 build

# Build Python module
.\bootstrap.ps1 python -i
```

## Quick Start

### CLI

```powershell
# List packages
pkg list
pkg list -L              # latest versions only

# Package information
pkg info maya

# Print environment
pkg env maya
pkg env maya -s          # with PKG_* stamp variables

# Launch application with environment
pkg env maya -- maya.exe
pkg env maya -- maya.exe -batch -file scene.ma

# Export environment to script
pkg env maya -o env.ps1

# Interactive shell
pkg shell

# Python REPL
pkg py
pkg py script.py
```

### Python API

```python
from pkg import Package, Env, Evar, App, Storage, Solver

# Scan packages
storage = Storage.scan()
print(f"Found {len(storage.packages)} packages")

# Resolve dependencies
solver = Solver(storage.packages)
solution = solver.solve("maya-2026.1.0")
# Returns: ["maya-2026.1.0", "redshift-3.5.0", "ocio-2.1.0", ...]

# Create package programmatically
p = Package("mytool", "1.0.0")
p.add_req("maya@>=2024")

env = Env("default")
env.add(Evar("MYTOOL_ROOT", "/opt/mytool", "set"))
env.add(Evar("PATH", "{MYTOOL_ROOT}/bin", "append"))
p.add_env(env)

app = App("mytool")
app.path = "/opt/mytool/bin/mytool"
p.add_app(app)
```

## Configuration

pkg uses Rez-compatible config loading via `rezconfig.py` (Python/YAML), with the
same precedence and overrides as Rez.

Config sources (low -> high priority):
1. `rezconfig.py` in the pkg Python module root
2. `REZ_CONFIG_FILE` list (paths separated by OS path separator)
3. `~/.rezconfig` (skipped if `REZ_DISABLE_HOME_CONFIG=1`)

`--cfg <path>` overrides `REZ_CONFIG_FILE` for this run.

Env overrides (highest priority):
1. `REZ_<KEY>` and JSON equivalents `REZ_<KEY>_JSON`
2. Plugin settings under `plugins.*` are NOT overridden by env vars (Rez behavior)

Value expansion:
1. `${ENV}` for environment variables
2. `{system.platform}`, `{system.os}`, `{system.arch}`, `{system.user}`, `{system.host}`
3. `~` and `~/` for home dir

Minimal example (`~/.rezconfig`):

```python
packages_path = ["D:/packages", "D:/tools"]
local_packages_path = "D:/packages-local"
release_packages_path = "//server/packages"
```

pkg-specific additions live under `plugins.pkg_rs.*` to preserve Rez schema. For example:

```python
plugins = {
    "pkg_rs": {
        "resolver_backend": "pkg",  # or "rez"
    }
}
```

## Package Structure

```
packages/
  maya/
    2024.0.0/
      package.py
    2025.0.0/
      package.py
  houdini/
    20.0.0/
      package.py
  redshift/
    3.5.0/
      package.py
```

## package.py Format

```python
from pkg import Package, Env, Evar, App
from pathlib import Path
import sys

def get_package():
    pkg = Package("houdini", "21.0.440")
    
    # Platform-specific paths
    if sys.platform == "win32":
        root = Path("C:/Program Files/Side Effects Software/Houdini 21.0.440")
    else:
        root = Path("/opt/hfs21.0.440")
    
    # Environment
    env = Env("default")
    env.add(Evar("HFS", str(root), "set"))
    env.add(Evar("PATH", "{HFS}/bin", "append"))
    env.add(Evar("HOUDINI_PATH", "{HFS}/houdini", "set"))
    pkg.add_env(env)
    
    # Applications
    exe = ".exe" if sys.platform == "win32" else ""
    pkg.add_app(App("houdini").with_path(f"{root}/bin/houdini{exe}"))
    pkg.add_app(App("hython").with_path(f"{root}/bin/hython{exe}"))
    
    # Dependencies
    pkg.add_req("redshift@>=3.5")
    pkg.add_req("ocio@2")
    
    return pkg
```

### Import Styles

Classes available in package.py via three methods:

```python
# Direct (injected into globals)
Package("tool", "1.0.0")

# Namespace
import pkg
pkg.Package("tool", "1.0.0")

# Explicit import
from pkg import Package, Env, Evar, App
```

## CLI Reference

```
pkg [OPTIONS] <COMMAND>

Commands:
  list        List available packages (alias: ls)
  info        Show package details
  env         Setup environment and run command
  graph       Dependency graph (DOT/Mermaid)
  scan        Scan package locations
  shell       Interactive mode (alias: sh)
  py          Python REPL
  gen-repo    Generate test repository
  gen-pkg     Generate package.py template
  completions Shell completions

Options:
  -r, --repo <PATH>   Package repository path
  -v                  Verbosity (-vv debug, -vvv trace)
  -x, --exclude       Exclude packages by pattern
  --json              JSON output
```

### Examples

```powershell
# Filter by name
pkg list -n maya

# Specific version info
pkg info maya-2024.0.0

# Dry-run (preview environment)
pkg env maya -n

# Dependency graph
pkg graph maya > deps.dot
dot -Tpng deps.dot -o deps.png

pkg graph maya -f mermaid

# Shell completions
pkg completions powershell >> $PROFILE
pkg completions bash >> ~/.bashrc
```

## Environment Variables

### Evar Actions

| Action   | Description                              |
|----------|------------------------------------------|
| `set`    | Set variable (overwrites existing)       |
| `append` | Append with path separator (`;` or `:`)  |
| `insert` | Prepend with path separator              |

### Token Expansion

Reference variables with `{VARNAME}`:

```python
env.add(Evar("MAYA_ROOT", "C:/Maya", "set"))
env.add(Evar("PATH", "{MAYA_ROOT}/bin", "append"))  # expands to C:/Maya/bin
```

## Version Constraints

| Constraint | Matches |
|------------|---------|
| `maya` | Latest version |
| `maya@2024` | 2024.x.x |
| `maya@>=2024` | 2024.0.0 and higher |
| `maya@>=2024,<2026` | 2024.x or 2025.x |
| `maya-2024.0.0` | Exact version |

## Configuration

### Package Locations

pkg reads package roots from Rez config (rezconfig.py + REZ_CONFIG_FILE + ~/.rezconfig).
Quick override via environment:

```powershell
# Windows
$env:REZ_PACKAGES_PATH = "C:\packages;D:\studio\packages"

# Linux
export REZ_PACKAGES_PATH="/opt/packages:/studio/packages"
```

Fallback: `repo/` in current directory if config paths are empty.

## Performance

Benchmarks on 200-package repository:

| Operation | Cold | Warm (cached) |
|-----------|------|---------------|
| Scan | 102ms | 31ms |
| Solve 25 requirements | 91μs | - |
| Solve chain depth 20 | 23μs | - |

Cache invalidation based on file mtime.

## API Reference

### Package

```python
pkg = Package("name", "1.0.0")
pkg.name          # "name-1.0.0"
pkg.base          # "name"
pkg.version       # "1.0.0"

pkg.add_req("other@>=1.0")
pkg.add_env(env)
pkg.add_app(app)
```

### Storage

```python
storage = Storage.scan()
storage = Storage.scan_paths([...])

storage.packages
storage.get("name-1.0.0")
storage.latest("name")
storage.versions("name")
storage.resolve("name@>=1.0")
```

### Solver

```python
solver = Solver(storage.packages)
solution = solver.solve("maya-2026.1.0")
solution = solver.solve_reqs(["maya", "houdini"])
```

### Env / Evar

```python
env = Env("default")
env.add(Evar("NAME", "value", "set"))
env.solve()     # expand tokens
env.commit()    # apply to process
```

### App

```python
app = App("maya")
app.path = "/path/to/maya"
app.env_name = "default"
app.args = ["-batch"]
app.cwd = "/project"
```

## Test Repository

Generate test packages for experimentation:

```powershell
pkg gen-repo -n 200 -V 5 --dep-rate 0.4 -o ./test-repo
$env:REZ_PACKAGES_PATH = "./test-repo"

pkg list
pkg env maya redshift -s
```

Test scripts in `tests/`:

```powershell
./tests/test.ps1 gen       # generate repositories
./tests/test.ps1 basic     # list, info, env
./tests/test.ps1 conflict  # dependency conflict scenarios
```

## License

MIT
