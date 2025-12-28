# USERGUIDE

packager-rs is a VFX/DCC package manager. You describe software in `package.py`,
then the CLI scans repositories, resolves dependencies, and builds an
environment to launch tools. Python is used only to author and execute
`package.py`; everything else is a single Rust binary.

## Quick Start (5 commands)

```powershell
# 1) Point pkg to your repo
$env:PKG_LOCATIONS="D:\packages;D:\tools"

# 2) Scan repositories
pkg scan

# 3) List packages
pkg list -L

# 4) Inspect a package
pkg info maya

# 5) Build env and launch tool
pkg env maya -- "C:\Program Files\Autodesk\Maya2024\bin\maya.exe"
```

Why this flow: you first tell the scanner where packages live, then verify
the data (`list`/`info`), then build an environment and run the software.

## Locations and Repositories

pkg scans for `package.py` under repositories. There are three ways to define
where to scan (in priority order):

1) `--repo` flags:

```powershell
pkg -r D:\packages -r \\server\repo list
```

2) `PKG_LOCATIONS` env var (split by `;` on Windows, `:` on Linux):

```powershell
$env:PKG_LOCATIONS="D:\packages;D:\tools"
```

3) Fallback: a `repo` folder in the current directory (if it exists).

You can also add a personal repo with `--user-packages`, which maps to
`~/packages`:

```powershell
pkg --user-packages list
```

## Scanning

```powershell
pkg scan
pkg scan D:\packages \\server\repo
```

`scan` builds the in-memory registry and cache. Use it to validate paths and
diagnose missing packages before running real commands.

## Finding Packages

```powershell
pkg list                # all packages
pkg list -L             # only latest versions
pkg list maya*          # glob patterns
pkg list -t dcc         # filter by tags
pkg info maya           # latest version details
pkg info maya-2024.0.0  # exact version
pkg info maya --json    # machine-readable output
```

Why `list` first: it shows what is actually installed and how names are
spelled, which avoids resolution errors later.

## Environments and Running Software

`pkg env` is the main way to build an environment. You can:
1) Print variables.
2) Export to a script.
3) Run a command with those variables.

```powershell
# Print env (raw or solved tokens)
pkg env maya
pkg env maya -s

# Export to a file (format chosen by extension)
pkg env maya -o env.ps1
pkg env maya -o env.cmd
pkg env maya -o env.sh

# Run a tool with the package environment
pkg env maya -- "C:\Program Files\Autodesk\Maya2024\bin\maya.exe"
```

Why `--`: everything after `--` is passed to the command, not to pkg.

### Multiple packages (ad-hoc toolset)

```powershell
pkg env maya redshift ocio -- "C:\Program Files\Autodesk\Maya2024\bin\maya.exe"
```

`pkg env` turns multiple packages into a temporary toolset, resolves deps, and
builds a merged environment. This is the simplest way to compose tools without
creating a dedicated toolset file.

### Output formats

```powershell
pkg env maya -f shell   # NAME=value (default)
pkg env maya -f export  # export NAME="value"
pkg env maya -f set     # set NAME=value
pkg env maya -f json    # JSON
```

## Dependency Graph

```powershell
pkg graph maya               # Graphviz DOT
pkg graph maya -f mermaid    # Mermaid
pkg graph maya -d 2          # limit depth
pkg graph maya -R            # reverse dependencies
```

Why use it: it shows why a package pulls in other tools and helps explain
version conflicts.

## Interactive Shell

The CLI shell is useful for fast exploration and includes extra commands
(`run`, `solve`) that are not available as top-level CLI commands.

```powershell
pkg shell
```

Inside the shell:

```
list, ls [patterns...]
info <package>
env <package> [app]
solve <package>
run [-f] <package> [app] [-- args...]
scan
```

Why use it: it keeps the scanned registry in memory, so repeated actions feel
instant and you can test `run/solve` interactively.

## Python REPL and Scripts

```powershell
pkg py                 # REPL with packager module loaded
pkg py script.py       # run a script
pkg py script.py -- -v # pass args
```

## Shell Completions

```powershell
pkg completions powershell >> $PROFILE
pkg completions bash >> ~/.bashrc
pkg completions zsh >> ~/.zshrc
pkg completions fish > ~/.config/fish/completions/pkg.fish
```

## Generate Test Repositories

```powershell
pkg gen-repo                 # default: medium
pkg gen-repo --small
pkg gen-repo --large
pkg gen-repo --stress
pkg gen-repo -n 100 -V 5     # custom size
pkg gen-repo -o ./my-repo
```

Why use it: stress‑test scanning/solving or demo the tool without real DCCs.

## Writing `package.py`

There is no `pkg gen-pkg` command in the current CLI. Create `package.py`
manually using this minimal template.

Directory layout:

```
packages/
  mytool/
    1.0.0/
      package.py
```

Minimal `package.py`:

```python
from packager import Package

def get_package():
    return Package("mytool", "1.0.0")
```

Typical `package.py`:

```python
from packager import Package, Env, Evar, App
from pathlib import Path
import sys

def get_package():
    pkg = Package("maya", "2024.0.0")

    root = Path("C:/Program Files/Autodesk/Maya2024") \
        if sys.platform == "win32" else Path("/usr/autodesk/maya2024")

    env = Env("default")
    env.add(Evar("MAYA_LOCATION", str(root), "set"))
    env.add(Evar("PATH", "{MAYA_LOCATION}/bin", "append"))
    env.add(Evar("PYTHONPATH", "{MAYA_LOCATION}/scripts", "append"))
    pkg.add_env(env)

    exe = ".exe" if sys.platform == "win32" else ""
    app = App("maya")
    app.path = str(root / "bin" / f"maya{exe}")
    pkg.add_app(app)

    pkg.add_req("arnold@>=5.0")
    pkg.add_req("redshift@>=3.5,<4.0")
    return pkg
```

Key rules:
- `get_package()` must return a `Package`.
- Use `Env` + `Evar` to define environment variables.
- Actions: `set`, `append`, `insert`.
- Tokens like `{MAYA_LOCATION}` are expanded by `pkg env -s`.

## Useful CLI Flags

```powershell
pkg -v ...        # info logs
pkg -vv ...       # debug logs
pkg -vvv ...      # trace logs
pkg -l            # log to pkg.log next to binary
pkg -l C:\tmp\pkg.log
pkg -x maya*      # exclude pattern (repeatable)
```

## Troubleshooting

- "Package not found": verify paths (`PKG_LOCATIONS`, `--repo`) and re-run `pkg scan`.
- "Environment not found": the package has no `Env` named `default` (set `--env-name`).
- "Failed to solve dependencies": run `pkg graph` to see conflicts; check your
  version constraints in `package.py`.
- "No executable path": the `App` entry has no `path`.
