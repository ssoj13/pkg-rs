# Plan 1: Rez Parity Implementation (build + pip)

## Goals
- Achieve functional parity with Rez for build and pip workflows.
- Preserve pkg-rs architecture while matching Rez behaviors where needed.

## Steps
1. Extend package schema and loader
   - Add `variants`, `hashed_variants`, `private_build_requires`, `pre_build_commands`, `requires_rez_version`.
   - Change `build_command` type to support `False | str | list`.
   - Update Python bindings and `python/pkg.pyi`.

2. Implement variant-aware build context
   - Compute variant requires and subpaths (hashed and non-hashed).
   - Resolve build context using `build_requires + private_build_requires + variant reqs`.
   - Align REZ_BUILD_* env vars with Rez semantics.

3. Build process & install pipeline
   - Add BuildProcess (local) and BuildSystem trait.
   - Implement per-variant build dirs and build.rxt snapshot.
   - Install payload, variant metadata (`variant.json`), and extra files.
   - Implement variant shortlinks when `hashed_variants` is true.

4. Build systems parity
   - Custom build: placeholder expansion, list commands, `build_command=False`, `parse_build_args.py` env export.
   - CMake build: generator settings, module path, REZ_BUILD_DOXYFILE, and build/install phases.
   - Make build: thread count, install target, child build args.

5. CLI parity
   - Add `--process`, `--fail-graph`, `--build-args`/`--child-build-args` parsing with `--`.
   - `--view-pre` to emit preprocessed package definition (Rez-like output).

6. Pip parity
   - Implement rezified python/pip discovery (resolver-based), enforce pip>=19.
   - Add install modes (min_deps/no_deps) and `--use-pep517` default.
   - Port distlib file mapping + `pip_install_remaps`.
   - Port PEP440 -> Rez requirement conversion.
   - Add hashed variants and pip metadata to generated package.

7. Tests
   - Build: custom, make, cmake with variants and install.
   - Pip: pure python + platform wheel, dependency resolution, entry points.

## Status
- 0/7 steps completed.
