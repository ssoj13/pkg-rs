# Package Structure

## Directory Layout

Packages are organized by name and version:

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
    21.0.0/
      package.py
  redshift/
    3.5.0/
      package.py
```

## Naming Convention

| Component | Example | Description |
|-----------|---------|-------------|
| Base name | `maya` | Package identifier |
| Version | `2024.0.0` | SemVer format |
| Full name | `maya-2024.0.0` | Unique identifier |

## Package Locations

Package locations are resolved in this order:

```powershell
# 1. CLI flag (highest priority)
pkg -r C:\packages list
pkg --repo /opt/packages list

# 2. rezconfig packages_path (override with REZ_PACKAGES_PATH)
$env:REZ_PACKAGES_PATH = "C:\pkg1;C:\pkg2"  # Windows
export REZ_PACKAGES_PATH="/opt/pkg1:/opt/pkg2"  # Linux

# 3. Fallback: repo/ folder in current directory
```

## Scanning Behavior

- Recursive search for `package.py` files
- Parallel directory walking (jwalk)
- Results cached with mtime invalidation
- Invalid packages logged as warnings
