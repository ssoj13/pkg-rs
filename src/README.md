# pkg_lib (Library)

## What It Is
The core library for pkg-rs. It defines package metadata, dependency solving, environment composition, repository scanning, and build/pip integration.

## How It Works
- Loads `package.py` definitions via the Python loader.
- Scans repositories and builds an index of packages.
- Resolves dependencies using the solver and merges environments.
- Provides build and pip import utilities for creating installable package repositories.

## Current Status
- Build pipeline supports `custom`, `make`, `cmake`, `cargo`, and `python`, with Rez-style `build_command` and `pre_build_commands` handling plus configure/build/install phases.
- Variants and hashed variants are supported, with `build.rxt` and `variant.json` emitted per variant and metadata copied on install.
- Pip import mirrors rez-pip layout: `pip install --target`, dist-info metadata parsing, entry-point wrappers, hashed variants, and dependency bundling when multiple dist-info are present.
- Repo configuration is driven by Rez config (`rezconfig.py`, `REZ_CONFIG_FILE`, `~/.rezconfig`) with `packages_path`, `local_packages_path`, and `release_packages_path`.
- Gaps vs Rez remain in pip conversion edge cases (full PEP440 coverage, rezified pip context) and some Rez-only features (release hooks, suite context integration).
