# pkg (CLI)

## What It Is
Command-line interface for pkg-rs. Provides environment setup, package inspection, repository scanning, build execution, and pip import.

## How It Works
- Delegates core logic to `pkg_lib`.
- Loads package definitions (`package.py`), resolves dependencies, and emits shell/env output.
- `pkg build` runs the build pipeline in the current package directory.
- `pkg pip` installs a Python package into a repository and generates a `package.py` wrapper.

## Current Status
- Build command supports Rez-style build variables, variants, hashed variants, `pre_build_commands`, and modular build systems (`custom`, `make`, `cmake`, `cargo`, `python`).
- Pip command matches rez-pip install layout (dist-info parsing, entry-point wrappers, hashed variants) and bundles dependencies when multiple dist-info are present.
- Repo configuration is available via `pkg-rs.toml` for scan paths and install targets.
- Some Rez-only flows (rezified pip/python context, release hooks, suite context) are not implemented.
