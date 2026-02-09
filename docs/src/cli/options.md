# Global Options

Options available for all commands.

## -r, --repo

Add package repository path. Can be repeated.

```powershell
pkg -r ./repo1 -r ./repo2 search
```

## -v (verbosity)

Control log verbosity.

| Flag | Level | Description |
|------|-------|-------------|
| (none) | warn | Warnings only |
| `-v` | info | Informational |
| `-vv` | debug | Debug details |
| `-vvv` | trace | Full trace |

## -l, --log

Log to file.

```powershell
pkg -l search              # Default: pkg.log
pkg -l mylog.txt search    # Custom path
```

## -x, --exclude

Exclude packages by pattern.

```powershell
pkg -x "test-*" search     # Exclude test packages
pkg -x old -x legacy search
```

## -u, --user-packages

Include user packages from `~/packages` (home directory).

```powershell
pkg -u search
```

## --json

JSON output (where supported).

```powershell
pkg search --json
pkg view maya --json
pkg env maya --json -n
```
