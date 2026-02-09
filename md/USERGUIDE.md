# USERGUIDE

pkg-rs is a VFX/DCC package manager. You describe software in `package.py`,
then the CLI scans repositories, resolves dependencies, and builds an
environment to launch tools. Python is used only to author and execute
`package.py`; everything else is a single Rust binary.

## Quick Start (5 commands)

```powershell
# 1) Point pkg to your repo
$env:REZ_PACKAGES_PATH="D:\packages;D:\tools"

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

## Configuration (rezconfig.py only)

Config is **only** `rezconfig.py` in the following locations (later overrides earlier):

1. `--cfg <path>` (single file)
2. `REZ_CONFIG_FILE` or `REZ_CONFIG_PATH` (list of paths)
3. `rezconfig.py` next to the pkg executable
4. `~/.pkg-rs/rezconfig.py` (created from embedded default if missing when no other config is set)

Defaults come from embedded `rezconfig.py`. Env overrides: `REZ_<KEY>` and `REZ_<KEY>_JSON` apply on top. No YAML/JSON config files.

Example `~/.pkg-rs/rezconfig.py`:

```python
packages_path = ["D:/packages", "D:/tools"]
local_packages_path = "D:/packages-local"
release_packages_path = "//server/packages"

# Resolver: "pkg" (PubGrub) or "rez" (Rez solver)
plugins = {"pkg_rs": {"resolver_backend": "pkg"}}

# Bind: add/remove modules for rez bind (e.g. extra platform/arch, or remove pip)
# bind_modules_extra = ["mymodule"]
# bind_modules_remove = ["rezgui"]
```

Config reference (under `plugins.pkg_rs`): `resolver_backend` (`"pkg"` | `"rez"`), `bind_modules_extra`, `bind_modules_remove`. See README for full bind/config details.

When using the **pkg** (PubGrub) resolver, Rez-style `package_filter` and `package_orderers` from config are applied if the builtin plugin is enabled (`plugins.pkg_rs.package_filters` / `plugins.pkg_rs.package_orderers` list includes `builtin` or `*`). These control which package versions are considered and their preference order during resolution.

## Locations and Repositories

pkg scans for `package.py` under repositories. Priority order:

1. `--repo` flags:

```powershell
pkg -r D:\packages -r \\server\repo list
```

2. rezconfig `packages_path` (including `REZ_PACKAGES_PATH`)

3. Fallback: a `repo` folder in the current directory (if it exists).

You can also add a personal repo with `--user-packages` (maps to `~/.pkg-rs/packages`):

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

### Rex: pre_commands, commands, post_commands

When you run `pkg env`, the resolved package (and its dependencies) can define
Rez-style command blocks in `package.py`: `pre_commands`, `commands`, `post_commands`.
These are executed in order (rex) and can change the environment (e.g. set or append
variables). After that, tokens like `{ROOT}` are expanded and the final env is
emitted or used for the command after `--`.

So: resolve → merge envs → run pre_commands → commands → post_commands → expand tokens → output/run.

### Multiple packages (ad-hoc toolset)

```powershell
pkg env maya redshift ocio -- "C:\Program Files\Autodesk\Maya2024\bin\maya.exe"
```

`pkg env` turns multiple packages into a temporary toolset, resolves deps, and
builds a merged environment. This is the simplest way to compose tools without
creating a dedicated toolset file.

### Output formats (shell plugins)

Formats follow shell semantics so you can source or eval the output:

```powershell
pkg env maya -f shell   # NAME=value (default; generic)
pkg env maya -f export  # export NAME="value" (bash/sh)
pkg env maya -f set     # set NAME=value (cmd.exe)
pkg env maya -f json    # JSON
```

With `-o <file>`, the extension picks the script format: `.cmd`/`.bat` (cmd), `.ps1` (PowerShell), `.sh` or other (bash). Append/insert actions are emitted correctly for each shell.

## Testing Packages

`pkg test` runs package tests for the resolved context:

```powershell
pkg test maya              # resolve maya, run pre_test_commands then tests
pkg test maya --inplace    # run tests in current dir using .rxt
```

Flow: resolve request → run `pre_test_commands` (rex) → run each entry in the
package `tests` section (e.g. executable or rex block). Use `pkg test` to
validate a package in a resolved environment.

## Build Packages

`pkg build` runs a build pipeline for the `package.py` in the current directory.
It detects the build system from the source tree or uses `--build-system`.

Supported build systems: `custom`, `make`, `cmake`, `cargo`, `python`.

Typical usage:

```powershell
pkg build --install
pkg build --install --process central
pkg build --build-system cargo --install
pkg build --build-args "--release"
```

Inputs:
- `package.py` (build metadata, variants, build_command)
- Source tree in the same directory
- CLI flags (`--build-args`, `--variants`, `--process`)

Outputs:
- Build directory (default `./build`)
- `build.rxt` and `variant.json`
- Installed package layout when `--install` is used

Install targets use config when available:
- `local_packages_path` for `--process local`
- `release_packages_path` for `--process central`
- fallback to `--prefix` or other scan roots

## Dependency Graph

```powershell
pkg graph maya               # Graphviz DOT
pkg graph maya -f mermaid    # Mermaid
pkg graph maya -d 2          # limit depth
pkg graph maya -R            # reverse dependencies
```

Why use it: it shows why a package pulls in other tools and helps explain
version conflicts.

## Suites and visibility

A **suite** is a directory that contains `suite.yaml` (created by `pkg rez suite --create DIR`). **Visible suites** are those that appear in your environment: we look at `PATH` and treat any parent directory of a `PATH` entry that contains `suite.yaml` as a visible suite. `pkg rez status` and `pkg rez suite --list` report these. Use suites to group tools and control which contexts are discoverable.

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
pkg py                 # REPL with pkg module loaded
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

Use `pkg gen-pkg` to create a template, or create `package.py` manually.

```powershell
pkg gen-pkg mytool-1.0.0
```

Directory layout:

```
packages/
  mytool/
    1.0.0/
      package.py
```

Minimal `package.py`:

```python
from pkg import Package

def get_package():
    return Package("mytool", "1.0.0")
```

Typical `package.py`:

```python
from pkg import Package, Env, Evar, App
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

Optional Rez-style command blocks (executed at `pkg env` or `pkg test`):

- `pre_commands`, `commands`, `post_commands` — run at `pkg env` (rex); can mutate env.
- `pre_test_commands` — run before tests at `pkg test`.
- `tests` — list of test entries (e.g. commands or rex) run by `pkg test`.

Use callables or string/list source; the loader captures them from `package.py`.

## Useful CLI Flags

```powershell
pkg -v ...        # info logs
pkg -vv ...       # debug logs
pkg -vvv ...      # trace logs
pkg -l            # log to pkg.log next to binary
pkg -l C:\tmp\pkg.log
pkg -x maya*      # exclude pattern (repeatable)
```

## Caches

- **Package cache**: Scan results are cached by path and mtime (`pkg.cache` next to the binary). Use `pkg rez pkg-cache --stats` / `--list` / `--clear`.
- **Memcache**: Rez-style resolve memcache is not active yet; `pkg rez memcache` shows config (`memcached_uri`) and status.

## Troubleshooting

- "Package not found": verify paths (`REZ_PACKAGES_PATH`/`packages_path`, `--repo`) and re-run `pkg scan`.
- "Environment not found": the package has no `Env` named `default` (set `--env-name`).
- "Failed to solve dependencies": run `pkg graph` to see conflicts; check your
  version constraints in `package.py`.
- "No executable path": the `App` entry has no `path`.
