# Quick Start

## Basic CLI Usage

```powershell
# List all packages
pkg list

# List latest versions only
pkg list -L

# Show package info
pkg info maya

# Resolve dependencies
pkg solve maya houdini

# Print environment
pkg env maya -s

# Run application
pkg run maya
```

## Creating Your First Package

1. Create directory structure:

```
mypackages/
  mytool/
    1.0.0/
      package.py
```

2. Write `package.py`:

```python
from packager import Package, Env, Evar, App

def get_package():
    pkg = Package("mytool", "1.0.0")
    
    # Environment
    env = Env("default")
    env.add(Evar("MYTOOL_ROOT", "/opt/mytool", "set"))
    env.add(Evar("PATH", "{MYTOOL_ROOT}/bin", "append"))
    pkg.add_env(env)
    
    # Application
    app = App("mytool")
    app.path = "/opt/mytool/bin/mytool"
    pkg.add_app(app)
    
    return pkg
```

3. Scan and use:

```powershell
pkg -r ./mypackages list
pkg -r ./mypackages info mytool
pkg -r ./mypackages run mytool
```

## Python Usage

```python
from packager import Storage, Solver, Package

# Scan packages
storage = Storage.scan()

# Find package
maya = storage.get("maya-2024.0.0")
print(f"Found: {maya.name}")

# Resolve dependencies
solver = Solver(storage.packages)
solution = solver.solve("maya-2024.0.0")
print("Resolved:", solution)
```
