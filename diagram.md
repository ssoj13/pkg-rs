# ASCII Diagrams

## Config Layering (Rez-Style)

```
Defaults (rezconfig.py or pkg-rs defaults)
  |
  v
Config list (REZ_CONFIG_FILE / PKG_RS_CONFIG list)
  |
  v
Home config (~/.rezconfig / ~/.pkg-rs/pkg-rs.toml)
  |
  v
Env overrides (REZ_* / PKG_*)
  |
  v
Env JSON overrides (REZ_*_JSON / PKG_*_JSON)
  |
  v
Package config section (build/release only)
  |
  v
Effective Config
```

## Resolve -> Context Dataflow

```
Package Requests + Filters + Orderers + Timestamp
  |
  v
Solver/Resolver
  |
  v
Resolved Packages + Variants
  |
  v
ResolvedContext (.rxt)
  |
  v
Shell Env Output / Command Execution / Suite
```

## Build Dataflow (Rez Parity)

```
Developer Package + Build Args
  |
  v
BuildSystem Detection (plugin)
  |
  v
BuildProcess (local/central)
  |
  v
Resolve Build Context (build_requires + private_build_requires)
  |
  v
Set REZ_BUILD_* env vars + pre_build_commands
  |
  v
Configure -> Build -> Install
  |
  +--> build.rxt + build-env scripts
  |
  v
Install payload + package metadata
```

## Pip Dataflow (Rez Parity)

```
Pip spec + CLI flags
  |
  v
Find rezified python/pip
  |
  v
pip install --target (temp)
  |
  v
Parse dist-info + RECORD + entry points
  |
  v
Convert PEP440 -> Rez requirements
  |
  v
Copy payload into repo layout
  |
  v
Write package.py + metadata
```

## Build Codepath (pkg-rs current)

```
CLI (pkg build)
  -> src/pkg/cli.rs
  -> src/pkg/commands/build.rs::cmd_build
  -> src/build.rs::build_package
      -> BuildSystemRegistry::new
      -> resolve_build_system
      -> collect_variants / select_variants
      -> create_build_env
      -> apply_pre_build_commands
      -> BuildSystem phases (configure/build/install)
```

## Pip Codepath (pkg-rs current)

```
CLI (pkg pip)
  -> src/pkg/cli.rs
  -> src/pkg/commands/pip.rs::cmd_pip
  -> src/pip.rs::import_pip_package
      -> find_python / ensure_pip
      -> run_pip_install
      -> load_metadata / parse_entry_points
      -> build_requirements / hash_variant_subpath
      -> copy_pip_payload / write_entry_points / write_package_py
```