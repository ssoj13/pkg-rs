//! CLI definitions for pkg command.

use clap::{Parser, Subcommand};
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
    Env {
        /// Package name(s)
        #[arg(required = true)]
        packages: Vec<String>,
        /// Command to run (after --)
        #[arg(last = true)]
        command: Vec<String>,
        /// Environment name (default: "default")
        #[arg(long)]
        env_name: Option<String>,
        /// Output format: shell, json, export, set
        #[arg(short, long, default_value = "shell")]
        format: String,
        /// Expand {TOKEN} references in values (default: true)
        #[arg(short, long, default_value = "true", action = clap::ArgAction::Set)]
        expand: bool,
        /// Write to file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Dry run (show what would happen)
        #[arg(short = 'n', long)]
        dry_run: bool,
        /// Add PKG_* stamp variables for each resolved package
        #[arg(short, long)]
        stamp: bool,
    },

    /// Build package in current directory
    Build {
        /// Clear current build before rebuilding
        #[arg(short = 'c', long)]
        clean: bool,
        /// Install the build to a package repository path
        #[arg(short = 'i', long)]
        install: bool,
        /// Install to a custom package repository path
        #[arg(short = 'p', long)]
        prefix: Option<PathBuf>,
        /// Build system to use (custom, make, cmake)
        #[arg(short = 'b', long = "build-system")]
        build_system: Option<String>,
        /// Build process to use (local, central)
        #[arg(long = "process", default_value = "local", value_parser = ["local", "central"])]
        process: String,
        /// Select variants to build (zero-indexed)
        #[arg(long = "variants")]
        variants: Vec<usize>,
        /// Arguments to pass to the build system
        #[arg(long = "build-args")]
        build_args: Option<String>,
        /// Arguments to pass to a child build system
        #[arg(long = "child-build-args")]
        child_build_args: Option<String>,
        /// Create build scripts instead of running the build
        #[arg(short = 's', long)]
        scripts: bool,
        /// Print preprocessed package definition and exit
        #[arg(long = "view-pre")]
        view_pre: bool,
        /// Extra build args after --
        #[arg(last = true)]
        extra_args: Vec<String>,
    },

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
    Pip {
        /// Package name, path, or URL to install
        package: String,
        /// Python version to use for pip (e.g., 3.11)
        #[arg(long = "python-version")]
        python_version: Option<String>,
        /// Do not install dependencies
        #[arg(long = "no-deps", conflicts_with = "min_deps")]
        no_deps: bool,
        /// Install minimal dependencies (default)
        #[arg(long = "min-deps", conflicts_with = "no_deps")]
        min_deps: bool,
        /// Install the package (required)
        #[arg(short = 'i', long)]
        install: bool,
        /// Install as released package
        #[arg(long)]
        release: bool,
        /// Install to a custom package repository path
        #[arg(short = 'p', long)]
        prefix: Option<PathBuf>,
        /// Extra args passed to pip install
        #[arg(long = "extra")]
        extra: Option<String>,
        /// Extra pip args after --
        #[arg(last = true)]
        extra_args: Vec<String>,
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
