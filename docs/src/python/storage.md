# Storage

Registry of available packages discovered from filesystem.

## Scanning

```python
from packager import Storage

# Scan default locations
storage = Storage.scan()

# Scan specific paths
storage = Storage.scan_paths(["/opt/packages", "./local"])
```

## Properties

| Property | Type | Description |
|----------|------|-------------|
| `packages` | list[Package] | All packages |
| `warnings` | list[str] | Load warnings |

## Methods

```python
# Get by exact name
pkg = storage.get("maya-2024.0.0")

# Get latest version
pkg = storage.latest("maya")

# Get all versions of a package
versions = storage.versions("maya")
# ["maya-2025.0.0", "maya-2024.0.0", ...]

# List base names
bases = storage.bases()
# ["maya", "houdini", "nuke", ...]

# Resolve with constraint
pkg = storage.resolve("maya@>=2024")

# Check existence
if storage.has("maya-2024.0.0"):
    ...

# Count
print(f"Found {storage.count()} packages")
```

## Example

```python
storage = Storage.scan()

# List all packages
for pkg in storage.packages:
    print(f"{pkg.name}: {len(pkg.reqs)} requirements")

# Find latest Maya
maya = storage.latest("maya")
if maya:
    print(f"Latest Maya: {maya.version}")

# Get all Houdini versions
for ver in storage.versions("houdini"):
    print(ver)
```

## Caching

Storage uses mtime-based caching:
- Cache file: `pkg.cache` (next to binary)
- Invalidation: automatic on file change
- First scan: ~100ms for 200 packages
- Cached scan: ~30ms
