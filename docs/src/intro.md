# Introduction

**packager-rs** is a software package manager designed for VFX and DCC pipelines. It manages software packages with Python-based definitions, resolves dependencies using a SAT solver, and configures runtime environments.

## Key Features

- **Python package definitions** - Full Python expressiveness in `package.py` files
- **SAT-based solver** - PubGrub algorithm for reliable dependency resolution
- **Environment management** - Variables with set/append/insert actions and token expansion
- **Fast scanning** - Parallel directory walking with mtime-based cache
- **Dual API** - CLI for operators, Python API for pipeline TDs

## Use Cases

- Managing DCC software (Maya, Houdini, Nuke, etc.)
- Render farm environment setup
- Plugin and tool deployment
- Development environment configuration
- CI/CD pipeline integration

## Architecture

```
+--------------------------------------------------+
|                   CLI / Python                    |
+--------------------------------------------------+
|  Storage  |  Loader  |  Solver  |   Launcher     |
+--------------------------------------------------+
|  Package  |   Env    |   Evar   |      App       |
+--------------------------------------------------+
```

- **Storage** - Scans filesystem for packages, maintains cache
- **Loader** - Parses package.py files using embedded Python
- **Solver** - Resolves dependencies using PubGrub algorithm
- **Launcher** - Sets up environment and launches applications
