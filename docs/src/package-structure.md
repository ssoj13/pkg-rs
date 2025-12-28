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

## Default Locations

**Windows:**
- `C:\packages`
- `%USERPROFILE%\.packager\packages`

**Linux:**
- `/opt/packages`
- `~/.packager/packages`

## Custom Locations

```powershell
# CLI flag
pkg -r C:\custom\packages list

# Environment variable
$env:PKG_LOCATIONS = "C:\pkg1;C:\pkg2"
pkg list
```

## Scanning Behavior

- Recursive search for `package.py` files
- Parallel directory walking (jwalk)
- Results cached with mtime invalidation
- Invalid packages logged as warnings
