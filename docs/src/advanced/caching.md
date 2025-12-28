# Caching

Package scanning uses mtime-based cache for performance.

## How It Works

1. First scan parses all `package.py` files
2. Results cached with file modification times
3. Subsequent scans check mtime before reparsing
4. Changed files are automatically reloaded

## Cache Location

Cache file: `pkg.cache` (next to binary)

```
C:\tools\pkg.exe
C:\tools\pkg.cache   <- JSON cache
```

## Performance

| Packages | Cold Scan | Warm Scan | Speedup |
|----------|-----------|-----------|---------|
| 50 | 25ms | 9ms | 2.8x |
| 100 | 83ms | 16ms | 5x |
| 200 | 102ms | 31ms | 3.3x |

## Cache Format

JSON with path -> (mtime, package) mapping:

```json
{
  "entries": {
    "/packages/maya/2024.0.0/package.py": {
      "mtime": 1703500000,
      "package": { "name": "maya-2024.0.0", ... }
    }
  }
}
```

## Invalidation

Cache entries invalidate when:

- File mtime changes (edit/save)
- File is deleted (pruned on next scan)
- Cache file is deleted (full rescan)

## Manual Clear

```powershell
# Delete cache file
Remove-Item (Get-Command pkg).Source.Replace('.exe', '.cache')

# Or just delete
rm C:\tools\pkg.cache
```

## Disabling Cache

Currently no flag to disable. Delete cache file for fresh scan.

## Parallel Scanning

Directory walking uses `jwalk` for parallel traversal:

- Multi-threaded directory enumeration
- Rayon-based parallelism
- Significant speedup on large repos
