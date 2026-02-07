# Rez Parity Audit Report (pkg-rs: build + pip)

Date: 2026-02-07

## Scope
- Compared pkg-rs build/pip pipelines against Rez reference implementation.
- Focused on functional parity for `rez-build` and `rez-pip` equivalents.
- Evidence is from local source files in `D:/_pkg-rs` and `D:/_pkg-rs/_ref/rez`.

## Evidence Sources (local)
- pkg-rs build: `D:/_pkg-rs/src/build.rs`
- pkg-rs pip: `D:/_pkg-rs/src/pip.rs`
- pkg-rs loader: `D:/_pkg-rs/src/loader.rs`
- pkg-rs build command type: `D:/_pkg-rs/src/build_command.rs`
- pkg-rs CLI build/pip: `D:/_pkg-rs/src/pkg/cli.rs`
- Rez build system: `D:/_pkg-rs/_ref/rez/src/rez/build_system.py`
- Rez custom build system: `D:/_pkg-rs/_ref/rez/src/rezplugins/build_system/custom.py`
- Rez CLI build: `D:/_pkg-rs/_ref/rez/src/rez/cli/build.py`
- Rez variant subpaths: `D:/_pkg-rs/_ref/rez/src/rez/package_resources.py`
- Rez pip: `D:/_pkg-rs/_ref/rez/src/rez/pip.py`
- Rez pip utils: `D:/_pkg-rs/_ref/rez/src/rez/utils/pip.py`

## TODO/FIXME Scan Summary
- pkg-rs: no TODO/FIXME/HACK markers detected in `D:/_pkg-rs` (excluding `_ref`).

## Implemented Parity (since last report)
- Variants + hashed variants in build pipeline.
- build_command accepts `False | str | list` with placeholder expansion.
- pre_build_commands extraction and execution, with env mutations applied to build env.
- REZ_BUILD_* variables are set, with absolute build/source paths.
- rez-pip baseline: pip>=19 check, `--use-pep517` default, RECORD-based copy, hashed variants.

## Findings (Remaining Gaps)

### 1) build.rxt + build scripts are not Rez-compatible
- Severity: High
- Evidence:
  - pkg-rs writes a custom JSON snapshot: `D:/_pkg-rs/src/build.rs:640-658`.
  - pkg-rs emits `build_env.*` scripts (not Rez `build-env`): `D:/_pkg-rs/src/build.rs:676-699`.
  - Rez custom build uses a `build-env` forwarding script and loads `build.rxt` as a ResolvedContext: `D:/_pkg-rs/_ref/rez/src/rezplugins/build_system/custom.py:110-126`, `D:/_pkg-rs/_ref/rez/src/rezplugins/build_system/custom.py:231-235`.
- Impact:
  - Rez tooling (or any Rez-compatible build shell workflow) cannot consume pkg-rs build artifacts.
  - `pkg build --scripts` does not match Rez behavior.
- Recommended fix:
  - Implement a Rez-compatible `build.rxt` (ResolvedContext JSON schema) and generate a `build-env` forwarding script.

### 2) Build process selection and release flags are missing
- Severity: Medium
- Evidence:
  - pkg-rs always sets `REZ_BUILD_TYPE=local`: `D:/_pkg-rs/src/build.rs:594`.
  - pkg-rs CLI has no `--process` option: `D:/_pkg-rs/src/pkg/cli.rs:117-148`.
  - Rez exposes `--process` and uses `config.default_build_process`: `D:/_pkg-rs/_ref/rez/src/rez/cli/build.py:36-40`.
  - Rez sets `REZ_IN_REZ_RELEASE` for central builds: `D:/_pkg-rs/_ref/rez/src/rez/build_system.py:250-253`.
- Impact:
  - No central/release build parity; environment flags differ from Rez.
- Recommended fix:
  - Add build process abstraction (local/central) and CLI parity, set `REZ_BUILD_TYPE` and `REZ_IN_REZ_RELEASE` accordingly.

### 3) pre_build_commands execution context is partial
- Severity: Medium
- Evidence:
  - pkg-rs binds a SimpleNamespace with limited fields: `D:/_pkg-rs/src/build.rs:966-1096`.
  - Rez binds a VariantBinding + RO_AttrDictWrapper for build context: `D:/_pkg-rs/_ref/rez/src/rez/build_system.py:260-294`.
- Impact:
  - Packages relying on full variant bindings or rex-style behavior can break.
- Recommended fix:
  - Implement a VariantBinding equivalent (or extend `this`/`build` bindings) and align env semantics to Rez.

### 4) parse_build_args.py and __PARSE_ARG_* exports are missing
- Severity: Medium
- Evidence:
  - Rez custom build exports args from `parse_build_args.py`: `D:/_pkg-rs/_ref/rez/src/rezplugins/build_system/custom.py:176-204`.
  - pkg-rs has no equivalent path in build execution (no exports in `build.rs`).
- Impact:
  - Custom build scripts that expect Rez-style `__PARSE_ARG_*` variables will fail.
- Recommended fix:
  - Load `parse_build_args.py` in build dir and export `__PARSE_ARG_*` into the build env.

### 5) Hashed variant shortlinks are not supported
- Severity: Low-Medium
- Evidence:
  - Rez supports `_v` shortlinks for hashed variants: `D:/_pkg-rs/_ref/rez/src/rez/package_resources.py:470-483`.
  - pkg-rs always uses the hash subpath directly: `D:/_pkg-rs/src/build.rs:302-319`.
- Impact:
  - No shortlink compatibility with Rez repositories that rely on `_v`.
- Recommended fix:
  - Implement optional shortlink creation/resolution with a config flag.

### 6) Pip interpreter discovery and dependency mode differ from Rez
- Severity: Medium
- Evidence:
  - pkg-rs finds python only on PATH: `D:/_pkg-rs/src/pip.rs:161-193`.
  - Rez searches rezified python/pip first and has min_deps/no_deps modes: `D:/_pkg-rs/_ref/rez/src/rez/pip.py:40-119`.
- Impact:
  - Wrong interpreter selection and dependency installation behavior vs Rez.
- Recommended fix:
  - Resolve python/pip via Storage/Solver and implement min_deps behavior.

### 7) Pip requirement conversion is simplified vs Rez PEP440 conversion
- Severity: High
- Evidence:
  - pkg-rs ignores `!=` and `===`, uses simplified semver conversion: `D:/_pkg-rs/src/pip.rs:750-776`.
  - Rez implements full PEP440 conversion with range unions: `D:/_pkg-rs/_ref/rez/src/rez/utils/pip.py:146-239`.
- Impact:
  - Dependency constraints are inaccurate; resolution diverges from Rez.
- Recommended fix:
  - Port Rez PEP440 conversion logic to Rust (including `!=` unions and wildcard rules).

### 8) Pip file mapping/remap behavior is incomplete
- Severity: Medium
- Evidence:
  - pkg-rs maps RECORD entries only to `python/` or `bin/`: `D:/_pkg-rs/src/pip.rs:622-645`.
  - Rez uses distlib + remap rules for installed files: `D:/_pkg-rs/_ref/rez/src/rez/pip.py:317-365`.
- Impact:
  - Incorrect payload layout and missing tools in some packages.
- Recommended fix:
  - Implement distlib-like RECORD mapping with configurable remaps.

### 9) Pip metadata attributes not preserved
- Severity: Medium
- Evidence:
  - pkg-rs writes only tags/description/env/apps: `D:/_pkg-rs/src/pip.rs:979-1030`.
  - Rez adds `pip_name`, `from_pip`, `is_pure_python`, `help`, `authors`, `tools`: `D:/_pkg-rs/_ref/rez/src/rez/pip.py:403-458`.
- Impact:
  - Missing provenance metadata and UI/UX info for pip packages.
- Recommended fix:
  - Extend Package schema or add custom attributes to persist pip metadata.

## Recommendation
Proceed with the parity-focused implementation plan in `plan2.md` and treat findings 1, 2, 6, and 7 as P0 blockers for Rez-equivalent behavior.