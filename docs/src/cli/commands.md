# Commands

All subcommands use the single `pkg` binary. Run `pkg --help` and `pkg <command> --help` for options.

## search

List or search packages.

```powershell
pkg search                # All packages
pkg search -L              # Latest versions only
pkg search maya*           # Glob patterns
pkg search -t dcc          # Filter by tag
pkg search --json          # JSON output
```

## view

Show package details (Rez: rez-view).

```powershell
pkg view maya              # Latest version
pkg view maya-2024.0.0     # Specific version
pkg view maya --json       # JSON output
```

## env

Print environment variables for package(s) or run a command with that environment.

```powershell
pkg env maya               # Print env (tokens expanded by default)
pkg env maya -e false      # Without token expansion
pkg env maya -s            # Include PKG_* stamp variables
pkg env maya -f json       # JSON format
pkg env maya -o env.ps1    # Export to file
pkg env maya bifrost arnold  # Multiple packages (toolset)
pkg env maya -- maya.exe   # Run command with env
```

**Options:**
- `-e, --expand` - Expand `{TOKEN}` references (default: true)
- `-s, --stamp` - Add PKG_* variables for each package (default: false)
- `-f, --format` - Output format: shell, json, export, set
- `-o, --output` - Write to file
- `-n, --dry-run` - Preview what would be set

**PATH order:** Direct requirements first (in request order), then transitive dependencies.

## depends

Dependency graph (Rez: rez-depends). Default format is list; use `-f dot` or `-f mermaid` for graphs.

```powershell
pkg depends maya           # List format
pkg depends maya -f dot    # Graphviz DOT
pkg depends maya -f mermaid  # Mermaid
pkg depends maya -R        # Reverse dependencies
pkg depends maya -d 2      # Limit depth
```

## build

Build the package in the current directory (package.py). Use `--install` to install to repo.

```powershell
pkg build
pkg build --install
pkg build --build-system cargo --install
pkg build --build-args "--release"
```

## pip

Import a PyPI package into the repository (Rez: rez-pip). Everything after the package name is passed to `pip install`.

```powershell
pkg pip install appdirs
pkg pip install appdirs -U --no-cache-dir
pkg pip appdirs -i
```

## bind

Bind system software as packages (Rez: rez-bind). Modules: platform, arch, os, python, rez, setuptools, pip.

```powershell
pkg bind --list            # List modules
pkg bind --search python   # Search by name
pkg bind python            # Bind one module
pkg bind --quickstart      # Bind all built-in
pkg bind --quickstart -r   # To release repo
```

## test

Run package tests (pre_test_commands + tests section).

```powershell
pkg test mypkg
pkg test mypkg --list
pkg test mypkg --inplace
```

## config

Show config paths and values.

```powershell
pkg config
pkg config --json
pkg config packages_path
pkg config --search-list
pkg config --source-list
```

## context, status, suite

- `pkg context` — create or load .rxt context (--print-request, --print-resolve, --format, --which).
- `pkg status` — show version, active context, visible suites.
- `pkg suite --list` — list suites; `pkg suite --create DIR` — create suite.

## cp, mv, rm, release

- `pkg cp` — copy package(s) between repos.
- `pkg mv` — move package(s).
- `pkg rm` — remove package(s).
- `pkg release` — release package to repo.

## diff, interpret, gui, version, completions

- `pkg diff` — compare two contexts.
- `pkg interpret` — run rex code.
- `pkg gui` — node editor GUI (graph, solve, export env).
- `pkg version` — version and build info.
- `pkg completions powershell|bash|zsh|fish` — generate shell completions.

## shell

Interactive mode with tab completion.

```powershell
pkg shell
pkg sh    # Alias
```

## python (py)

Python REPL with pkg module loaded, or run a script.

```powershell
pkg python                  # REPL
pkg py                      # Alias
pkg py script.py            # Run script
pkg py script.py -- -v      # With arguments
```

## Other Rez-style commands

- `pkg bundle` — bundle context to dir/zip (bin-patch).
- `pkg benchmark` — resolve benchmark.
- `pkg yaml2py` — convert package.yaml to package.py.
- `pkg pkg-cache` — package cache stats/clear/list.
- `pkg pkg-ignore` — ignore patterns.
- `pkg memcache` — memcache status (stub).
- `pkg plugins` — list plugins.
- `pkg selftest` — run self-tests.
- `pkg help` — show usage.
