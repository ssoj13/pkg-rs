# Plan 2 - Rez Parity Completion (build + pip)

1. Build: emit Rez-compatible `build.rxt` (ResolvedContext schema) and generate a `build-env` forwarding script that spawns a build shell from `build.rxt`.
2. Build: add build process abstraction (local/central), expose `--process` in CLI, set `REZ_BUILD_TYPE` and `REZ_IN_REZ_RELEASE` accordingly.
3. Build: align `pre_build_commands` context with Rez (VariantBinding-like `this`, RO_AttrDictWrapper-like `build`), and add `parse_build_args.py` support with `__PARSE_ARG_*` exports.
4. Build: optional hashed variant shortlinks (`_v`) with config flag and resolution logic.
5. Pip: resolve python/pip via Storage/Solver (rezified python/pip first) and implement min_deps/no_deps behavior.
6. Pip: port full PEP440 -> Rez requirement conversion (including `!=` unions and wildcard rules).
7. Pip: implement distlib/RECORD mapping with configurable remaps (`pip_install_remaps`) for payload layout.
8. Pip: persist pip metadata fields (`pip_name`, `from_pip`, `is_pure_python`, `help`, `authors`, `tools`) in Package and emit them in generated `package.py`.

## Test Plan
- Build: use example package with variants and pre_build_commands; verify `build.rxt` load + `build-env` behavior matches Rez.
- Build: test `--process local|central` flags and REZ_* variables.
- Pip: import a package with markers and `!=` constraints; verify requirements match Rez output.
- Pip: verify RECORD mapping and entry points in repo layout.
