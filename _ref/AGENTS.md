# Agents

## Rez-Build Dataflow (ASCII)

[User CLI + CWD]
  |
  v
[rez.cli._entry_points.run_rez_build]
  |
  v
[rez.cli._main.run("build")]
  |
  v
[rez.cli.build.command]
  |
  +--> Load DeveloperPackage (CWD)
  |
  +--> Select BuildSystem plugin (package build_system/build_command or auto-detect)
  |
  +--> Create BuildProcess plugin (default: local)
           |
           v
        Build variants:
           - create ResolvedContext -> build.rxt
           - run BuildSystem.build
           - optionally install payload + update package.py

## Rez-Build Codepaths (ASCII)

rez-build
  -> rez.cli._entry_points.run_rez_build
    -> rez.cli._main.run
      -> rez.cli._util.subcommands["build"] (arg_mode=grouped)
        -> rez.cli.build.setup_parser / command
          -> rez.build_system.create_build_system
          -> rez.build_process.create_build_process
            -> rezplugins.build_process.local.LocalBuildProcess.build
              -> LocalBuildProcess._build_variant_base
                -> BuildSystem.build

## Rez-Pip Dataflow (ASCII)

[CLI: rez-pip]
  |
  v
[rez.cli.pip.command]
  |
  v
[rez.pip.pip_install_package]
  |
  +--> find_pip (rez python/pip or fallback)
  +--> pip install --target <temp>
  +--> distlib: collect distributions
  +--> translate pip requirements -> rez requirements
  +--> make_package + copy files (python/, bin/)
  +--> install into packages_path (local/release/prefix)

## Key Files
- src/rez/cli/_entry_points.py
- src/rez/cli/_main.py
- src/rez/cli/_util.py
- src/rez/cli/build.py
- src/rez/build_system.py
- src/rez/build_process.py
- src/rezplugins/build_process/local.py
- src/rezplugins/build_system/custom.py
- src/rez/cli/pip.py
- src/rez/pip.py
- src/rez/utils/pip.py
- docs/source/pip.rst
