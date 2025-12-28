# Version Constraints

Flexible version specification for dependencies.

## Syntax

| Pattern | Matches | Example |
|---------|---------|---------|
| `pkg` | Latest | `maya` -> `maya-2025.0.0` |
| `pkg@X.Y.Z` | Exact | `maya@2024.0.0` |
| `pkg@X` | Major | `maya@2024` -> `2024.*.*` |
| `pkg@X.Y` | Minor | `maya@2024.1` -> `2024.1.*` |
| `pkg@>=X.Y` | Min | `maya@>=2024.0` |
| `pkg@<X.Y` | Max | `maya@<2025.0` |
| `pkg@>=X,<Y` | Range | `maya@>=2024,<2026` |

## Examples

```python
# Any version
pkg.add_req("python")

# Exact version
pkg.add_req("cuda@11.8.0")

# Major version (any 3.x.x)
pkg.add_req("redshift@3")

# Minimum version
pkg.add_req("arnold@>=5.0")

# Version range
pkg.add_req("ocio@>=2.0,<3.0")

# Complex constraint
pkg.add_req("numpy@>=1.20,<2.0")
```

## SemVer Compatibility

Versions follow [Semantic Versioning](https://semver.org/):

- **MAJOR** - Incompatible API changes
- **MINOR** - Backwards-compatible features
- **PATCH** - Backwards-compatible fixes

## Resolution Order

When multiple versions match, the solver prefers:

1. Highest matching version
2. Already-resolved version (for shared deps)
3. First available if no preference

## CLI Usage

```powershell
# Resolve with constraints
pkg solve "maya@>=2024" "redshift@3"

# Check what version resolves
pkg info maya@2024
```
