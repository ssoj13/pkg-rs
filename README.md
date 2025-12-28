# pkg-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

Package manager for VFX/DCC applications: Maya, Houdini, Nuke, and everything else in your pipeline.

Start here: see [USERGUIDE.md](USERGUIDE.md) for step-by-step workflows.

## Why?

You have Maya 2024, Maya 2025, three versions of Houdini, Redshift that works only with certain versions, and a dozen studio tools. Each needs specific environment variables, paths, and dependencies. This tool:

1. **Knows what works together** - request "maya + redshift + studio_tools" and it finds compatible versions automatically. If there's a conflict, it explains why in plain English
2. **Sets up the environment** - PATH, PYTHONPATH, license servers, plugin paths - all configured and ready
3. **Launches apps correctly** - `pkg run maya` and you're in, with everything loaded

## Features

- **Embedded Python** - package definitions are Python files, but you don't need Python installed. Interpreter is built-in
- **Smart dependency solver** - finds compatible versions automatically. Conflicts? You'll know exactly which packages disagree and why
- **Environment management** - variables, paths, tokens like `{MAYA_ROOT}/bin` that expand correctly
- **Fast** - parallel scanning, JSON cache, rescans in milliseconds
- **CLI + Python API** - use from terminal or embed in your pipeline scripts
- **Cross-platform** - Windows and Linux

## Why not rez?

[rez](https://github.com/AcademySoftwareFoundation/rez) is the industry standard and it's great. But:

| | pkg-rs | rez |
|---|--------|-----|
| Install | single binary, no dependencies | Python + pip + system packages |
| Speed | 30ms warm scan, 90μs solve | seconds |
| Python for packages | embedded, always works | system Python, version conflicts possible |
| Conflict messages | "maya-2024 needs redshift>=3.5, but you requested redshift-3.0" | sometimes cryptic |
| Windows | native | works but painful |

pkg-rs is not a rez replacement for large studios with existing infrastructure. It's for smaller teams, freelancers, or anyone who wants something that just works out of the box.

## Installation

```powershell
cargo install pkg-rs
```

That's it. Single binary, no dependencies (no Python required).

### From source

```powershell
# Build CLI
.\bootstrap.ps1 build

# Build Python module (for embedding in pipeline scripts)
.\bootstrap.ps1 python -i
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

# Python REPL with pkg module exposed
pkg py

# Python interpreter can also execute files
pkg py package.py
```

### Python

```python
from pkg import Package, Env, Evar, App, Storage, Solver

# Scan packages
storage = Storage.scan()
print(f"Found {len(storage.packages)} packages")

# Resolve dependencies - returns list of package names
solver = Solver(storage.packages)
solution = solver.solve("maya-2026.1.0")
# solution = ["maya-2026.1.0", "redshift-3.5.0", "ocio-2.1.0", ...]

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
from pkg import Package, Env, Evar, App
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
  py          Python REPL with pkg
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

### Package Locations

Set `PKG_LOCATIONS` to point to your package repositories:

```powershell
# Windows
$env:PKG_LOCATIONS = "C:\packages;D:\studio\packages"

# Linux
export PKG_LOCATIONS="/opt/packages:/studio/packages"
```

If `PKG_LOCATIONS` is not set, pkg looks for a `repo/` folder in the current directory.

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

## Try It Out

Don't want to spend a week setting up test packages? There's a built-in repository generator:

```powershell
# Generate 200 packages with realistic dependencies
pkg gen-repo -n 200 -V 5 --dep-rate 0.4 -o ./test-repo

# Point to it
$env:PKG_LOCATIONS = "./test-repo"

# Play around
pkg list
pkg solve maya houdini redshift
pkg env maya -s
```

Or use the ready-made test scripts in `tests/`:

```powershell
# Windows
./tests/test.ps1 gen       # generate test repos
./tests/test.ps1 basic     # list, info, env, solve
./tests/test.ps1 conflict  # dependency conflict scenarios

# Linux/macOS
./tests/test.sh gen
./tests/test.sh basic
./tests/test.sh conflict
```

The generator creates packages with names like `maya`, `houdini`, `redshift`, `arnold`, `usd` — familiar VFX software with plausible version numbers and dependency chains.

## License

MIT
