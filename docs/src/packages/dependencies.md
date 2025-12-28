# Dependencies

Packages can declare dependencies on other packages.

## Adding Requirements

```python
pkg.add_req("redshift")           # Any version
pkg.add_req("arnold@>=5.0")       # Minimum version
pkg.add_req("ocio@2")             # Major version 2.x
pkg.add_req("python@>=3.9,<3.12") # Version range
```

## Constraint Syntax

| Syntax | Meaning |
|--------|---------|
| `pkg` | Latest version |
| `pkg@1.0.0` | Exactly 1.0.0 |
| `pkg@1` | Any 1.x.x |
| `pkg@>=1.0` | 1.0.0 or higher |
| `pkg@<2.0` | Below 2.0.0 |
| `pkg@>=1.0,<2.0` | Range |

## Resolution

Dependencies are resolved at runtime:

```python
# After solving
pkg.solve(storage.packages)

# pkg.deps contains resolved versions
print(pkg.deps)  # ["redshift-3.5.2", "arnold-5.3.0", ...]
```

## Conflict Detection

The solver detects version conflicts:

```
Resolution failed:
  Version conflict for ocio:
    maya-2024.0.0 requires ocio@>=2.0
    legacy-tool-1.0.0 requires ocio@<2.0
```

## Transitive Dependencies

Dependencies are resolved transitively:

```
maya@2024 -> redshift@>=3.5 -> cuda@>=11
                            -> optix@>=7
          -> arnold@>=5.0   -> cuda@>=11 (shared)
```

The solver finds a `cuda` version satisfying both.

## CLI

```powershell
# Preview resolved environment
pkg env maya houdini -n

# JSON output
pkg env maya --json -n
```
