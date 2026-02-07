# Rez-Build Dataflow and Codepaths (ASCII)

Dataflow (rez-build)

[User CLI args + CWD]
  |
  v
[rez.cli._entry_points.run_rez_build]
  |
  v
[rez.cli._main.run("build")]
  |
  v
[Arg parsing: rez.cli._util.subcommands["build"] (grouped args)]
  |
  v
[rez.cli.build.command]
  |
  +--> [Load DeveloperPackage from CWD]
  |        |
  |        v
  |     [Package metadata (build_system/build_command, variants, requires)]
  |
  +--> [Select BuildSystem plugin]
  |        |
  |        +--> explicit build_system/build_command in package
  |        +--> or auto-detect via build files
  |
  +--> [Create BuildProcess plugin (default: local)]
           |
           v
        [For each variant]
           |
           +--> [Resolve build context (ResolvedContext)]
           |        |
           |        +--> write build.rxt
           |
           +--> [BuildSystem.build]
           |        |
           |        +--> set REZ_BUILD_* env vars
           |        +--> run pre_build_commands if present
           |        +--> execute build command/build tool
           |
           +--> [If install]
                    |
                    +--> copy payload to install path
                    +--> write variant.json (if variant indexed)
                    +--> install extra files (build.rxt, etc)
                    +--> run pre_install tests (if configured)
                    +--> update package.py in repo

Outputs
- Build directory under `build_directory` (default: "build")
- build.rxt (resolved build context)
- variant.json (per variant, if indexed)
- Optional build-env script (when --scripts is used)
- Installed package payload and updated package.py in target repo

Codepaths (rez-build)

rez-build
  -> rez.cli._entry_points.run_rez_build
    -> rez.cli._main.run
      -> rez.cli._util.subcommands["build"] (arg_mode = grouped)
        -> rez.cli.build.setup_parser / command
          -> rez.build_system.create_build_system
            -> build_system plugin (custom/cmake/make/...)
          -> rez.build_process.create_build_process
            -> build_process plugin (local by default)
              -> LocalBuildProcess.build
                -> LocalBuildProcess._build_variant_base
                  -> BuildSystem.build
