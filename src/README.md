# pkg_lib (Library)

## What It Is
The core library for pkg-rs. It defines package metadata, dependency solving, environment composition, repository scanning, and build/pip integration.

## How It Works
- Loads `package.py` definitions via the Python loader.
- Scans repositories and builds an index of packages.
- Resolves dependencies using the solver and merges environments.
- Provides build and pip import utilities for creating installable package repositories.

## Current Status
- Build pipeline supports `custom`, `make`, and `cmake`, with Rez-style `build_command` and `pre_build_commands` handling.
- Variants and hashed variants are supported, with `build.rxt` and `variant.json` emitted per variant.
- Pip import mirrors rez-pip layout: `pip install --target`, dist-info metadata parsing, entry-point wrappers, and hashed variants.
- Gaps vs Rez remain in pip conversion (full PEP440 coverage, rezified pip context) and some Rez-only features (central build process, variant shortlinks).
