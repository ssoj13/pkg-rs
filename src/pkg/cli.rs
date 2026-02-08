//! CLI definitions for pkg command.

use clap::{Args, Parser, Subcommand};
use clap_complete::Shell as CompletionShell;
use std::path::PathBuf;

/// pkg - Software package management
#[derive(Parser)]
#[command(name = "pkg")]
#[command(author, version)]
#[command(help_template = "{about-section}\n{usage-heading} {usage}\n\n{all-args}\n\n{after-help}")]
#[command(about = "pkg - Software package manager for VFX pipelines.\n\n\
    Manages packages with Python-based definitions (package.py),\n\
    resolves dependencies using SAT solver, and configures environments.\n\n\
    EXAMPLES:\n\
    \x20 pkg env maya                Print environment\n\
    \x20 pkg env maya -- maya.exe    Launch with environment\n\
    \x20 pkg build                   Build package in current directory\n\
    \x20 pkg pip <pkg> -i           Import pip package into repo\n\
    \x20 pkg python                  Run embedded Python REPL")]
#[command(after_help = "SUBCOMMAND OPTIONS:\n\
    Each command has its own options. Use 'pkg <command> --help' to see them:\n\
    \x20 pkg env --help              Environment options (-s/--stamp, -e/--expand)\n\
    \x20 pkg build --help            Build options (-b/--build-system)\n\
    \x20 pkg search --help           Search options (-L, --tag, --json)")]
pub struct Cli {
    /// Verbosity: -v (info), -vv (debug), -vvv (trace)
    #[arg(short = 'v', action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Log to file (default: pkg.log next to binary)
    #[arg(short = 'l', long = "log", global = true)]
    pub log_file: Option<Option<PathBuf>>,

    /// Config file override (Rez config .py/.yaml)
    #[arg(short = 'c', long = "config", alias = "cfg")]
    pub cfg: Option<PathBuf>,

    /// Package repositories (can be specified multiple times)
    #[arg(short = 'r', long = "repo", global = true)]
    pub repos: Vec<PathBuf>,

    /// Exclude packages matching pattern (can repeat)
    #[arg(short = 'x', long = "exclude", global = true)]
    pub exclude: Vec<String>,

    /// Include user packages (~/.pkg-rs/packages)
    #[arg(short = 'u', long = "user-packages", global = true, default_value = "false")]
    pub user_packages: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct EnvArgs {
    /// Package name(s)
    #[arg(required = true)]
    pub(crate) packages: Vec<String>,
    /// Command to run (after --)
    #[arg(last = true)]
    pub(crate) command: Vec<String>,
    /// Environment name (default: "default")
    #[arg(long)]
    pub(crate) env_name: Option<String>,
    /// Output format: shell, json, export, set
    #[arg(short, long, default_value = "shell")]
    pub(crate) format: String,
    /// Expand {TOKEN} references in values (default: true)
    #[arg(short, long, default_value = "true", action = clap::ArgAction::Set)]
    pub(crate) expand: bool,
    /// Write to file
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
    /// Dry run (show what would happen)
    #[arg(short = 'n', long)]
    pub(crate) dry_run: bool,
    /// Add PKG_* stamp variables for each resolved package
    #[arg(short, long)]
    pub(crate) stamp: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct BuildArgs {
    /// Clear current build before rebuilding
    #[arg(short = 'c', long)]
    pub(crate) clean: bool,
    /// Install the build to a package repository path
    #[arg(short = 'i', long)]
    pub(crate) install: bool,
    /// Install to a custom package repository path
    #[arg(short = 'p', long)]
    pub(crate) prefix: Option<PathBuf>,
    /// Build system to use (custom, make, cmake, cargo, python)
    #[arg(short = 'b', long = "build-system")]
    pub(crate) build_system: Option<String>,
    /// Build process to use (local, central)
    #[arg(long = "process", value_parser = ["local", "central"])]
    pub(crate) process: Option<String>,
    /// Select variants to build (zero-indexed)
    #[arg(long = "variants")]
    pub(crate) variants: Vec<usize>,
    /// Arguments to pass to the build system
    #[arg(long = "build-args", allow_hyphen_values = true)]
    pub(crate) build_args: Option<String>,
    /// Arguments to pass to a child build system
    #[arg(long = "child-build-args", allow_hyphen_values = true)]
    pub(crate) child_build_args: Option<String>,
    /// Create build scripts instead of running the build
    #[arg(short = 's', long)]
    pub(crate) scripts: bool,
    /// Print preprocessed package definition and exit
    #[arg(long = "view-pre")]
    pub(crate) view_pre: bool,
    /// Extra build args after --
    #[arg(last = true)]
    pub(crate) extra_args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct PipArgs {
    /// Package name, path, or URL to install
    pub(crate) package: String,
    /// Python version to use for pip (e.g., 3.11)
    #[arg(long = "python-version")]
    pub(crate) python_version: Option<String>,
    /// Do not install dependencies
    #[arg(long = "no-deps", conflicts_with = "min_deps")]
    pub(crate) no_deps: bool,
    /// Install minimal dependencies (default)
    #[arg(long = "min-deps", conflicts_with = "no_deps")]
    pub(crate) min_deps: bool,
    /// Install the package (required)
    #[arg(short = 'i', long)]
    pub(crate) install: bool,
    /// Install as released package
    #[arg(long)]
    pub(crate) release: bool,
    /// Install to a custom package repository path
    #[arg(short = 'p', long)]
    pub(crate) prefix: Option<PathBuf>,
    /// Extra args passed to pip install
    #[arg(long = "extra")]
    pub(crate) extra: Option<String>,
    /// Extra pip args after --
    #[arg(last = true)]
    pub(crate) extra_args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct RezConfigArgs {
    /// Output dict/list field values as JSON
    #[arg(long = "json")]
    pub(crate) json: bool,
    /// List config files searched
    #[arg(long = "search-list")]
    pub(crate) search_list: bool,
    /// List config files sourced
    #[arg(long = "source-list")]
    pub(crate) source_list: bool,
    /// Print value of a specific setting (dot path)
    pub(crate) field: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct SearchArgs {
    /// Name patterns (glob: maya, cinem*, *_ext?)
    pub(crate) patterns: Vec<String>,
    /// Filter by tags (can repeat)
    #[arg(short = 't', long = "tag")]
    pub(crate) tags: Vec<String>,
    /// Show only latest versions
    #[arg(short = 'L', long)]
    pub(crate) latest: bool,
    /// Output as JSON
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct ViewArgs {
    /// Package name
    pub(crate) package: String,
    /// Output as JSON
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct DependsArgs {
    /// Package name(s)
    pub(crate) packages: Vec<String>,
    /// Output format: dot, mermaid, list
    #[arg(short, long, default_value = "list")]
    pub(crate) format: String,
    /// Maximum depth (0 = unlimited)
    #[arg(short, long, default_value = "0")]
    pub(crate) depth: usize,
    /// Show reverse dependencies
    #[arg(short = 'R', long)]
    pub(crate) reverse: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct DiffArgs {
    /// Package to diff
    pub(crate) pkg1: String,
    /// Package to diff against (defaults to previous version)
    pub(crate) pkg2: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct InterpretArgs {
    /// Output format: shell name, dict, or table
    #[arg(short, long)]
    pub(crate) format: Option<String>,
    /// Interpret in an empty environment
    #[arg(long = "no-env")]
    pub(crate) no_env: bool,
    /// Parent variables to update rather than overwrite (or "all")
    #[arg(long = "pv", alias = "parent-variables")]
    pub(crate) parent_vars: Vec<String>,
    /// File containing rex code
    pub(crate) file: PathBuf,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct CpArgs {
    /// Package repository destination path
    #[arg(long = "dest-path")]
    pub(crate) dest_path: Option<PathBuf>,
    /// Package search paths (PATHS separated by os path separator)
    #[arg(long = "paths")]
    pub(crate) paths: Option<String>,
    /// Don't search local packages
    #[arg(long = "no-local", alias = "nl")]
    pub(crate) no_local: bool,
    /// Copy to a different package version
    #[arg(long = "reversion")]
    pub(crate) reversion: Option<String>,
    /// Copy to a different package name
    #[arg(long = "rename")]
    pub(crate) rename: Option<String>,
    /// Overwrite existing package/variants
    #[arg(short = 'o', long = "overwrite")]
    pub(crate) overwrite: bool,
    /// Shallow copy (symlink top-level entries)
    #[arg(short = 's', long = "shallow")]
    pub(crate) shallow: bool,
    /// Follow symlinks instead of copying symlink entries
    #[arg(long = "follow-symlinks")]
    pub(crate) follow_symlinks: bool,
    /// Keep timestamp of source package
    #[arg(short = 'k', long = "keep-timestamp")]
    pub(crate) keep_timestamp: bool,
    /// Copy even if not relocatable
    #[arg(short = 'f', long = "force")]
    pub(crate) force: bool,
    /// Allow copying into empty repository
    #[arg(long = "allow-empty")]
    pub(crate) allow_empty: bool,
    /// Dry run (no changes)
    #[arg(long = "dry-run")]
    pub(crate) dry_run: bool,
    /// Select variants to copy (zero-indexed)
    #[arg(long = "variants")]
    pub(crate) variants: Vec<usize>,
    /// Copy variant with given URI (not yet supported)
    #[arg(long = "variant-uri")]
    pub(crate) variant_uri: Option<String>,
    /// Package to copy
    pub(crate) pkg: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct MvArgs {
    /// Package repository destination path
    #[arg(short = 'd', long = "dest-path")]
    pub(crate) dest_path: PathBuf,
    /// Keep timestamp of source package
    #[arg(short = 'k', long = "keep-timestamp")]
    pub(crate) keep_timestamp: bool,
    /// Move even if not relocatable
    #[arg(short = 'f', long = "force")]
    pub(crate) force: bool,
    /// Package to move (name-version)
    pub(crate) pkg: String,
    /// Repository containing the package (optional)
    pub(crate) path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct RmArgs {
    /// Remove the specified package (name-version)
    #[arg(short = 'p', long = "package")]
    pub(crate) package: Option<String>,
    /// Remove the specified package family (name only)
    #[arg(short = 'f', long = "family")]
    pub(crate) family: Option<String>,
    /// Force remove package family even if not empty
    #[arg(long = "force-family")]
    pub(crate) force_family: bool,
    /// Remove packages ignored for >= DAYS
    #[arg(short = 'i', long = "ignored-since")]
    pub(crate) ignored_since: Option<i64>,
    /// Dry run mode (ignored-since only)
    #[arg(long = "dry-run")]
    pub(crate) dry_run: bool,
    /// Repository containing the package(s)
    pub(crate) path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct PkgIgnoreArgs {
    /// Unignore a package
    #[arg(short = 'u', long = "unignore")]
    pub(crate) unignore: bool,
    /// Allow ignoring missing packages
    #[arg(short = 'a', long = "allow-missing")]
    pub(crate) allow_missing: bool,
    /// Package to (un)ignore (name-version)
    pub(crate) pkg: String,
    /// Repository containing the package (optional)
    pub(crate) path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct PluginsArgs {
    /// Package search paths (PATHS separated by os path separator)
    #[arg(long = "paths")]
    pub(crate) paths: Option<String>,
    /// Package to list plugins for
    pub(crate) pkg: String,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct PkgCacheArgs {
    /// Clear cache file
    #[arg(long = "clear")]
    pub(crate) clear: bool,
    /// Show cache stats
    #[arg(long = "stats")]
    pub(crate) stats: bool,
    /// List cache entries
    #[arg(long = "list")]
    pub(crate) list: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct MemcacheArgs {
    /// Clear resolve cache (if enabled)
    #[arg(long = "clear")]
    pub(crate) clear: bool,
    /// Show memcache status
    #[arg(long = "stats")]
    pub(crate) stats: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct ReleaseArgs {
    /// Release message
    #[arg(short = 'm', long = "message")]
    pub(crate) message: Option<String>,
    /// Force the vcs type to use (currently only git)
    #[arg(long = "vcs")]
    pub(crate) vcs: Option<String>,
    /// Allow release of version earlier than latest
    #[arg(long = "no-latest")]
    pub(crate) no_latest: bool,
    /// Ignore existing tag (git)
    #[arg(long = "ignore-existing-tag")]
    pub(crate) ignore_existing_tag: bool,
    /// Skip repository errors (git)
    #[arg(long = "skip-repo-errors")]
    pub(crate) skip_repo_errors: bool,
    /// Do not prompt for release message
    #[arg(long = "no-message")]
    pub(crate) no_message: bool,
    /// Build system to use (custom, make, cmake, cargo, python)
    #[arg(short = 'b', long = "build-system")]
    pub(crate) build_system: Option<String>,
    /// Build process to use (local, central)
    #[arg(long = "process", value_parser = ["local", "central"])]
    pub(crate) process: Option<String>,
    /// Select variants to build (zero-indexed)
    #[arg(long = "variants")]
    pub(crate) variants: Vec<usize>,
    /// Arguments to pass to the build system
    #[arg(long = "build-args", allow_hyphen_values = true)]
    pub(crate) build_args: Option<String>,
    /// Arguments to pass to a child build system
    #[arg(long = "child-build-args", allow_hyphen_values = true)]
    pub(crate) child_build_args: Option<String>,
    /// Extra build args after --
    #[arg(last = true)]
    pub(crate) extra_args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct SelftestArgs {
    /// Verbose output
    #[arg(short, long)]
    pub(crate) verbose: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TestArgs {
    /// List package's tests and exit
    #[arg(short = 'l', long = "list")]
    pub(crate) list: bool,
    /// Dry-run mode
    #[arg(long = "dry-run")]
    pub(crate) dry_run: bool,
    /// Stop on first test failure
    #[arg(short = 's', long = "stop-on-fail")]
    pub(crate) stop_on_fail: bool,
    /// Run tests in the current environment
    #[arg(long = "inplace")]
    pub(crate) inplace: bool,
    /// Extra packages to add to test environment
    #[arg(long = "extra-packages")]
    pub(crate) extra_packages: Vec<String>,
    /// Set package search path (os path separator)
    #[arg(long = "paths")]
    pub(crate) paths: Option<String>,
    /// Don't load local packages
    #[arg(long = "no-local", alias = "nl")]
    pub(crate) no_local: bool,
    /// Package to run tests on
    pub(crate) pkg: String,
    /// Tests to run (run all if not provided)
    pub(crate) tests: Vec<String>,
    /// Extra args after -- (only with a single test)
    #[arg(last = true)]
    pub(crate) extra_args: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct Yaml2pyArgs {
    /// Path to yaml or directory containing package.yaml
    pub(crate) path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct BundleArgs {
    /// Leave non-relocatable packages non-bundled
    #[arg(short = 's', long = "skip-non-relocatable")]
    pub(crate) skip_non_relocatable: bool,
    /// Bundle even if not relocatable
    #[arg(short = 'f', long = "force")]
    pub(crate) force: bool,
    /// Don't apply library patching within the bundle
    #[arg(short = 'n', long = "no-lib-patch")]
    pub(crate) no_lib_patch: bool,
    /// Context to bundle (.rxt)
    pub(crate) rxt: PathBuf,
    /// Destination directory (must not exist)
    pub(crate) dest_dir: PathBuf,
    /// Write bundle as a directory instead of zip
    #[arg(long = "dir")]
    pub(crate) dir: bool,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct BenchmarkArgs {
    /// Output dir
    #[arg(long = "out", default_value = "out")]
    pub(crate) out: PathBuf,
    /// Run every resolve N times and take the average
    #[arg(long = "iterations", default_value = "1")]
    pub(crate) iterations: usize,
    /// Show histogram from results in --out
    #[arg(long = "histogram")]
    pub(crate) histogram: bool,
    /// Compare RESULTS_DIR to results in --out
    #[arg(long = "compare")]
    pub(crate) compare: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct RezStubArgs {
    /// Additional args passed to command (not implemented yet)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) args: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// rez env
    Env(EnvArgs),
    /// rez build
    Build(BuildArgs),
    /// Spawn a build environment from build.rxt (internal)
    #[command(name = "build-env", hide = true)]
    BuildEnv {
        /// Build directory containing build.rxt
        #[arg(long = "build-path")]
        build_path: PathBuf,
        /// Variant index (optional)
        #[arg(long = "variant-index")]
        variant_index: Option<usize>,
        /// Install flag (affects REZ_BUILD_INSTALL)
        #[arg(long = "install")]
        install: bool,
        /// Install path (optional)
        #[arg(long = "install-path")]
        install_path: Option<PathBuf>,
    },
    /// rez pip
    Pip(PipArgs),
    /// rez bind
    Bind(RezStubArgs),
    /// rez config
    Config(RezConfigArgs),
    /// rez context
    Context(RezStubArgs),
    /// rez cp
    Cp(CpArgs),
    /// rez depends
    Depends(DependsArgs),
    /// rez diff
    Diff(DiffArgs),
    /// rez gui
    Gui,
    /// rez help
    Help,
    /// rez interpret
    Interpret(InterpretArgs),
    /// rez memcache
    Memcache(MemcacheArgs),
    /// rez pkg-cache
    PkgCache(PkgCacheArgs),
    /// rez plugins
    Plugins(PluginsArgs),
    /// rez python
    #[command(name = "python", visible_alias = "py")]
    Python {
        /// Python script to run (omit for REPL)
        script: Option<PathBuf>,
        /// Arguments passed to script (after --)
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// rez release
    Release(ReleaseArgs),
    /// rez search
    Search(SearchArgs),
    /// rez selftest
    Selftest(SelftestArgs),
    /// rez status
    Status(RezStubArgs),
    /// rez suite
    Suite(RezStubArgs),
    /// rez test
    Test(TestArgs),
    /// rez view
    View(ViewArgs),
    /// rez yaml2py
    Yaml2py(Yaml2pyArgs),
    /// rez bundle
    Bundle(BundleArgs),
    /// rez benchmark
    Benchmark(BenchmarkArgs),
    /// rez pkg-ignore
    PkgIgnore(PkgIgnoreArgs),
    /// rez mv
    Mv(MvArgs),
    /// rez rm
    Rm(RmArgs),

    /// Show version and build info
    Version,

    /// Generate shell completions
    Completions {
        /// Shell type
        shell: CompletionShell,
    },

    /// Legacy pkg-rs commands (hidden)
    #[command(name = "legacy", hide = true, subcommand)]
    Legacy(LegacyCommands),
}

#[derive(Subcommand)]
pub(crate) enum LegacyCommands {
    /// List available packages
    #[command(visible_alias = "ls")]
    List {
        /// Name patterns (glob: maya, cinem*, *_ext?)
        patterns: Vec<String>,
        /// Filter by tags (can repeat)
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
        /// Show only latest versions
        #[arg(short = 'L', long)]
        latest: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show package details
    Info {
        /// Package name
        package: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show dependency graph
    Graph {
        /// Package name(s)
        packages: Vec<String>,
        /// Output format: dot, mermaid
        #[arg(short, long, default_value = "dot")]
        format: String,
        /// Maximum depth (0 = unlimited)
        #[arg(short, long, default_value = "0")]
        depth: usize,
        /// Show reverse dependencies
        #[arg(short = 'R', long)]
        reverse: bool,
    },

    /// Scan locations for packages
    Scan {
        /// Paths to scan
        paths: Vec<PathBuf>,
    },

    /// Generate test repository with random packages
    #[command(name = "gen-repo", after_help =
        "PRESETS:\n  \
        --small   10 packages x 2 versions = 20 nodes\n  \
        --medium  50 packages x 3 versions = 150 nodes [default]\n  \
        --large   200 packages x 5 versions = 1000 nodes\n  \
        --stress  1000 packages x 10 versions = 10000 nodes"
    )]
    GenerateRepo {
        /// Output directory
        #[arg(short, long, default_value = "./test-repo")]
        output: PathBuf,
        /// Small preset
        #[arg(long, conflicts_with_all = ["medium", "large", "stress", "packages", "versions"])]
        small: bool,
        /// Medium preset (default)
        #[arg(long, conflicts_with_all = ["small", "large", "stress", "packages", "versions"])]
        medium: bool,
        /// Large preset
        #[arg(long, conflicts_with_all = ["small", "medium", "stress", "packages", "versions"])]
        large: bool,
        /// Stress preset
        #[arg(long, conflicts_with_all = ["small", "medium", "large", "packages", "versions"])]
        stress: bool,
        /// Number of packages
        #[arg(short = 'n', long)]
        packages: Option<usize>,
        /// Versions per package
        #[arg(short = 'V', long)]
        versions: Option<usize>,
        /// Maximum dependency depth
        #[arg(short, long, default_value = "3")]
        depth: usize,
        /// Dependency probability (0.0-1.0)
        #[arg(long, default_value = "0.3")]
        dep_rate: f64,
        /// Random seed
        #[arg(long)]
        seed: Option<u64>,
    },

    /// Generate package.py template
    #[command(name = "gen-pkg")]
    GenPkg {
        /// Package identifier: name-version[--variant] (version starts at first `-` + digit)
        /// Examples: maya-2026.1.0, my-plugin-1.0.0, maya-2026.1.0--win64
        package_id: String,
    },

    /// Interactive shell with tab-completion
    #[command(visible_alias = "sh")]
    Shell,

    /// Launch graphical interface
    #[command(name = "gui")]
    Gui,
}
