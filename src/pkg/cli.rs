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
    \x20 pkg ls                      List all packages\n\
    \x20 pkg ls -L                   Only latest versions\n\
    \x20 pkg info maya               Show package details\n\
    \x20 pkg env maya                Print environment\n\
    \x20 pkg env maya -- maya.exe    Launch with environment\n\
    \x20 pkg sh                      Interactive mode")]
#[command(after_help = "SUBCOMMAND OPTIONS:\n\
    Each command has its own options. Use 'pkg <command> --help' to see them:\n\
    \x20 pkg env --help              Environment options (-s/--stamp, -e/--expand)\n\
    \x20 pkg list --help             Filtering options (-L, --tags, --json)\n\
    \x20 pkg graph --help            Graph options (--format, --depth)")]
pub struct Cli {
    /// Verbosity: -v (info), -vv (debug), -vvv (trace)
    #[arg(short = 'v', action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Log to file (default: pkg.log next to binary)
    #[arg(short = 'l', long = "log", global = true)]
    pub log_file: Option<Option<PathBuf>>,

    /// Config file override (Rez config .py/.yaml)
    #[arg(long = "cfg", global = true)]
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
    #[arg(long = "process", default_value = "local", value_parser = ["local", "central"])]
    pub(crate) process: String,
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

#[derive(Subcommand)]
pub enum Commands {
    /// Run Python REPL or execute script with pkg module
    #[command(name = "py")]
    Python {
        /// Python script to run (omit for REPL)
        script: Option<PathBuf>,
        /// Arguments passed to script (after --)
        #[arg(last = true)]
        args: Vec<String>,
    },

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

    /// Setup environment and optionally run command
    Env(EnvArgs),

    /// Build package in current directory
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

    /// Install a pip package into a repository
    Pip(PipArgs),

    /// Rez-compatible command group (rez env/build/pip/...)
    #[command(name = "rez", subcommand)]
    Rez(RezCommands),

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
        /// Package identifier: name-version[-variant]
        /// Examples: maya-2026.1.0, my-plugin-1.0.0-win64
        package_id: String,
    },

    /// Show version and build info
    Version,

    /// Interactive shell with tab-completion
    #[command(visible_alias = "sh")]
    Shell,

    /// Generate shell completions
    Completions {
        /// Shell type
        shell: CompletionShell,
    },

    /// Launch graphical interface
    #[command(name = "gui")]
    Gui,
}

#[derive(Subcommand, Debug)]
#[command(disable_help_subcommand = true)]
pub(crate) enum RezCommands {
    /// rez env
    Env(EnvArgs),
    /// rez build
    Build(BuildArgs),
    /// rez pip
    Pip(PipArgs),
    /// rez bind
    #[command(name = "bind")]
    Bind(RezStubArgs),
    /// rez config
    #[command(name = "config")]
    Config(RezConfigArgs),
    /// rez context
    #[command(name = "context")]
    Context(RezStubArgs),
    /// rez cp
    #[command(name = "cp")]
    Cp(RezStubArgs),
    /// rez depends
    #[command(name = "depends")]
    Depends(RezStubArgs),
    /// rez diff
    #[command(name = "diff")]
    Diff(RezStubArgs),
    /// rez gui
    #[command(name = "gui")]
    Gui(RezStubArgs),
    /// rez help
    #[command(name = "help")]
    Help(RezStubArgs),
    /// rez interpret
    #[command(name = "interpret")]
    Interpret(RezStubArgs),
    /// rez memcache
    #[command(name = "memcache")]
    Memcache(RezStubArgs),
    /// rez pkg-cache
    #[command(name = "pkg-cache")]
    PkgCache(RezStubArgs),
    /// rez plugins
    #[command(name = "plugins")]
    Plugins(RezStubArgs),
    /// rez python
    #[command(name = "python")]
    Python(RezStubArgs),
    /// rez release
    #[command(name = "release")]
    Release(RezStubArgs),
    /// rez search
    #[command(name = "search")]
    Search(RezStubArgs),
    /// rez selftest
    #[command(name = "selftest")]
    Selftest(RezStubArgs),
    /// rez status
    #[command(name = "status")]
    Status(RezStubArgs),
    /// rez suite
    #[command(name = "suite")]
    Suite(RezStubArgs),
    /// rez test
    #[command(name = "test")]
    Test(RezStubArgs),
    /// rez view
    #[command(name = "view")]
    View(RezStubArgs),
    /// rez yaml2py
    #[command(name = "yaml2py")]
    Yaml2py(RezStubArgs),
    /// rez bundle
    #[command(name = "bundle")]
    Bundle(RezStubArgs),
    /// rez benchmark
    #[command(name = "benchmark")]
    Benchmark(RezStubArgs),
    /// rez pkg-ignore
    #[command(name = "pkg-ignore")]
    PkgIgnore(RezStubArgs),
    /// rez mv
    #[command(name = "mv")]
    Mv(RezStubArgs),
    /// rez rm
    #[command(name = "rm")]
    Rm(RezStubArgs),
    /// rez _rez-complete (placeholder)
    #[command(name = "_rez-complete")]
    Complete(RezStubArgs),
    /// rez _rez_fwd (placeholder)
    #[command(name = "_rez_fwd")]
    Forward(RezStubArgs),
}

#[derive(Args, Debug)]
pub(crate) struct RezStubArgs {
    /// Additional args passed to rez-* (not implemented yet)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) args: Vec<String>,
}
