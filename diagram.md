# ASCII Diagrams

## Build Dataflow

```
package.py + source tree + CLI flags + repos
  |
  v
Load Package -> Collect Variants -> Select Variant(s)
  |
  v
Resolve Build Context -> Set REZ_BUILD_* env
  |
  v
Execute pre_build_commands (env mutations)
  |
  v
Run Build System (custom/make/cmake) or emit build scripts
  |
  +--> build.rxt snapshot + variant.json
  |
  v
Install payload + package.py + variant metadata (optional)
```

## Build Codepath

```
CLI (pkg build)
  -> src/pkg/cli.rs
  -> src/pkg/commands/build.rs::cmd_build
  -> src/build.rs::build_package
      -> resolve_build_system
      -> collect_variants / select_variants
      -> create_build_env / build_env_vars
      -> apply_pre_build_commands
      -> write_build_snapshot / write_variant_marker
      -> run_custom_build | run_make_build | run_cmake_build
      -> install_package_files / install_variant_metadata
```

## Pip Dataflow

```
package spec + CLI flags + repos
  |
  v
Find Python/Pip -> pip install --target (temp)
  |
  v
Parse dist-info metadata -> Build rez requirements / variants
  |
  v
Copy payload into repo layout (hashed variant subpath)
  |
  v
Generate entry points -> Write package.py
```

## Pip Codepath

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
