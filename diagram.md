# ASCII Diagrams

## Config Layering (Rez-Style)

```
Defaults (rezconfig.py)
  |
  v
Config list (REZ_CONFIG_FILE list)
  |
  v
Home config (~/.rezconfig, skip if REZ_DISABLE_HOME_CONFIG)
  |
  v
Env overrides (REZ_*, plugins excluded)
  |
  v
Env JSON overrides (REZ_*_JSON)
  |
  v
Package config section (build/release only)
  |
  v
Effective Config (Rez schema + plugins.pkg_rs.*)
```

## Embedded Python Layout

```
python/ (sys.path root)
  |
  +-- rez/         (rezconfig.py, resolved_context.py)
  +-- rezplugins/  (build/shell/repo plugins)
```

## Rez Commands (Single Binary)

```
pkg binary
  |
  v
subcommands
  |
  +-- env (pkg rez env) -> cmd_env
  +-- build (pkg rez build) -> cmd_build
  +-- pip (pkg rez pip) -> cmd_pip
  +-- rez <cmd> stubs (parity TODO)
```

## Resolve -> Context Dataflow

```
Package Requests + Filters + Orderers + Timestamp
  |
  v
Backend Select (plugins.pkg_rs.resolver_backend)
  |------------------------------|
  v                              v
Pkg Solver (PubGrub)       Rez Resolver (python)
  |                              |
  +--------------+---------------+
                 v
Resolved Packages + Variants
  |
  v
ResolvedContext (.rxt)
  |
  v
Shell Env Output / Command Execution / Suite
```

## Env Pipeline (Current)

```
pkg env
  |
  v
Resolve deps
  |
  v
Package._env/default
  |
  v
Stamp PKG_* -> Env.solve_impl
  |
  v
Emit/commit env
  |
  v
NOTE: pre/commands/post/pre_test are not executed.
```

## Env Pipeline (Target Rez Parity)

```
pkg env
  |
  v
Resolve deps + variants
  |
  v
ResolvedContext
  |
  v
Execute pre/commands/post (rex) -> Env mutations
  |
  v
Emit/commit env
  |
  v
pre_test_commands + tests -> Test report
```

## Package Loader Commands Capture

```
package.py source
  |
  v
Loader.execute_package_py
  |
  +--> Python exec (globals)
  |       |
  |       +--> Extract pre_build/pre/commands/post/pre_test
  |             callable -> source
  |             string/list -> text
  |
  +--> get_package() -> Package/dict
  |
  v
Merge extracted commands into Package (if missing)
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
