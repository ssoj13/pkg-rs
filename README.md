# packager-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

VFX/DCC package manager with Python-based definitions, SAT-based dependency resolution, and environment management.

Start here: see `USERGUIDE.md` for step-by-step workflows (scan, list, env, run, package.py).

## Features

- **Python package definitions** - Define packages in `package.py` files with full Python expressiveness
- **SAT-based solver** - PubGrub algorithm for reliable dependency resolution with conflict explanation
- **Environment management** - Variables with Set/Append/Insert actions and token expansion
- **Fast scanning** - Parallel directory walking (jwalk) with JSON cache for quick rescans
- **CLI and Python API** - Use from command line or embed in Python scripts
- **Cross-platform** - Windows and Linux support

## Installation

```powershell
# Build CLI
.\bootstrap.ps1 build

# Build and install Python module
.\bootstrap.ps1 python -i

# Run tests
.\bootstrap.ps1 test

# Run benchmarks
.\bootstrap.ps1 bench
```

## Quick Start

### CLI

```powershell
# List all packages
pkg list

# List only latest versions
pkg list -L

# Show package details
pkg info maya

# Resolve dependencies
pkg solve maya houdini

# Print environment (with token expansion)
pkg env maya -s

# Launch application
pkg run maya

# Interactive shell
pkg shell
```

### Python

```python
from pkg import Package, Env, Evar, App, Storage, Solver

# Scan packages
storage = Storage.scan()
print(f"Found {len(storage.packages)} packages")

# Resolve dependencies
solver = Solver(storage.packages)
solution = solver.solve("maya-2026.1.0")
print("Resolved:", solution)

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

### package.py Import Styles

In `package.py` files, classes are available in three ways:

```python
# 1. Direct access (classes injected into globals)
Package("tool", "1.0.0")  # Just works

# 2. Namespace style
import pkg
pkg.Package("tool", "1.0.0")

# 3. Explicit import
from pkg import Package, Env, Evar, App, Action
```

All three styles work - use whichever you prefer.

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
from packager import Package, Env, Evar, App
from pathlib import Path
import sys

def get_package():
    pkg = Package("houdini", "21.0.440")
    
    # Platform paths
    root = Path("C:/Program Files/Side Effects Software/Houdini 21.0.440") \
           if sys.platform == "win32" else Path("/opt/hfs21.0.440")
    
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

## CLI Reference

```
pkg [OPTIONS] <COMMAND>

Commands:
  list        List available packages
  info        Show package details
  solve       Resolve dependencies
  env         Print environment variables
  run         Launch application
  graph       Show dependency graph (DOT/Mermaid)
  scan        Scan locations for packages
  shell       Interactive mode (tab completion)
  py          Python REPL with packager
  gen-repo    Generate test repository
  completions Generate shell completions

Options:
  -r, --repo <PATH>   Add package repository
  -v                  Verbose (-vv debug, -vvv trace)
  -x, --exclude       Exclude packages by pattern
  --json              JSON output (where supported)
```

### Examples

```powershell
# List packages filtered by name
pkg list -n maya

# Show specific version
pkg info maya-2024.0.0

# Dry-run solve (preview)
pkg solve maya houdini -n

# Export environment to file
pkg env maya -o env.ps1

# Run with extra arguments
pkg run maya -- -batch -file scene.ma

# Dependency graph (Graphviz DOT)
pkg graph maya > deps.dot
dot -Tpng deps.dot -o deps.png

# Dependency graph (Mermaid)
pkg graph maya -f mermaid

# Generate shell completions
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

Reference other variables with `{VARNAME}`:

```python
env.add(Evar("MAYA_ROOT", "C:/Maya", "set"))
env.add(Evar("PATH", "{MAYA_ROOT}/bin", "append"))  # -> C:/Maya/bin
env.add(Evar("PYTHONPATH", "{MAYA_ROOT}/scripts", "append"))
```

## Version Constraints

Use `@` syntax for version requirements:

| Constraint | Matches |
|------------|---------|
| `maya` | Latest version |
| `maya@2024` | 2024.x.x |
| `maya@>=2024` | 2024.0.0 and higher |
| `maya@>=2024,<2026` | 2024.x or 2025.x |
| `maya-2024.0.0` | Exact version |

## Configuration

### Environment Variables

- `PKG_LOCATIONS` - Additional search paths (`;` on Windows, `:` on Linux)

### Default Locations

**Windows:**
- `C:\packages`
- `%USERPROFILE%\.packager\packages`

**Linux:**
- `/opt/packages`
- `~/.packager/packages`

## Performance

Benchmarks on typical VFX repository (200 packages):

| Operation | Cold | Warm (cached) |
|-----------|------|---------------|
| Scan 200 packages | 102ms | 31ms |
| Solve 25 requirements | 91us | - |
| Solve chain depth 20 | 23us | - |

Cache uses mtime-based invalidation - modified packages are automatically reloaded.

## API Reference

### Package

```python
pkg = Package("name", "1.0.0")
pkg.name          # "name-1.0.0" (full name)
pkg.base          # "name"
pkg.version       # "1.0.0"
pkg.reqs          # requirements (constraints)
pkg.deps          # resolved dependencies

pkg.add_req("other@>=1.0")
pkg.add_env(env)
pkg.add_app(app)
pkg.solve(storage.packages)
```

### Storage

```python
storage = Storage.scan()              # Default locations
storage = Storage.scan_paths([...])   # Custom paths

storage.packages       # All packages
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
# Returns: ["maya-2026.1.0", "houdini-21.0.0", "redshift-3.5.0", ...]
```

### Env / Evar

```python
env = Env("default")
env.add(Evar("NAME", "value", "set"))   # set/append/insert
env.solve()     # Expand tokens
env.commit()    # Apply to current process
```

### App

```python
app = App("maya")
app.path = "/path/to/maya"
app.env_name = "default"
app.args = ["-batch"]
app.cwd = "/project"
```

## License

MIT
