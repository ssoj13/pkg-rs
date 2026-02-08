//! Build pipeline utilities.
//!
//! The build pipeline resolves build-time requirements, constructs a build
//! environment, executes the selected build system, and optionally installs
//! the resulting package to a repository layout. This module follows Rez
//! semantics for variants and build environment variables.

use crate::dep::DepSpec;
use crate::error::BuildError;
use crate::{Env, Evar, Package, Storage};
mod systems;
mod msvc;
use msvc::{ensure_msvc_env, MsvcEnvState};
use systems::{BuildContext, BuildPhase, BuildSystemArgs, BuildSystemRegistry};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyTuple};
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Options for running a package build.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Override build system (custom/make/cmake/cargo/python).
    pub build_system: Option<String>,
    /// Arguments passed to the selected build system.
    pub build_args: Vec<String>,
    /// Extra arguments passed to a child build system (e.g. cmake --build --).
    pub child_build_args: Vec<String>,
    /// Variant indices to build (zero-indexed).
    pub variants: Vec<usize>,
    /// Remove build and install directories before running.
    pub clean: bool,
    /// Install build artifacts into a repository layout.
    pub install: bool,
    /// Optional install prefix (repository root).
    pub prefix: Option<PathBuf>,
    /// Emit build env scripts instead of executing the build.
    pub scripts: bool,
    /// Build process type (local or central).
    pub build_type: BuildType,
    /// Extra args after `--` (used by parse_build_args.py and build args).
    pub extra_args: Vec<String>,
}

/// Build process type (local or central).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildType {
    Local,
    Central,
}

impl BuildType {
    fn as_str(self) -> &'static str {
        match self {
            BuildType::Local => "local",
            BuildType::Central => "central",
        }
    }
}

/// Summary of a completed build.
#[derive(Debug, Clone)]
pub struct BuildReport {
    /// Number of variants built.
    pub built_variants: usize,
    /// Absolute build directory path.
    pub build_root: PathBuf,
    /// Install directory if install was requested.
    pub install_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct BuildVariant {
    index: Option<usize>,
    requires: Vec<String>,
    subpath: Option<String>,
}

#[derive(Serialize)]
struct ResourceHandleSnapshot {
    key: String,
    variables: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct ResolvedContextSnapshot {
    serialize_version: String,
    timestamp: i64,
    requested_timestamp: i64,
    building: bool,
    testing: bool,
    caching: bool,
    implicit_packages: Vec<String>,
    package_requests: Vec<String>,
    package_paths: Vec<String>,
    append_sys_path: bool,
    package_caching: bool,
    package_cache_async: bool,
    default_patch_lock: String,
    patch_locks: HashMap<String, String>,
    package_orderers: Option<serde_json::Value>,
    package_filter: Vec<serde_json::Value>,
    graph: String,
    resolved_packages: Vec<ResourceHandleSnapshot>,
    resolved_ephemerals: Vec<String>,
    rez_version: String,
    rez_path: String,
    user: String,
    host: String,
    platform: String,
    arch: String,
    os: String,
    created: i64,
    parent_suite_path: Option<String>,
    suite_context_name: Option<String>,
    status: String,
    failure_description: Option<String>,
    from_cache: bool,
    solve_time: f64,
    load_time: f64,
    num_loaded_packages: i64,
    #[serde(rename = "pkg_env")]
    pkg_env: HashMap<String, String>,
}

#[derive(Clone, Serialize)]
struct ResolvedPackageSnapshot {
    name: String,
    base: String,
    version: String,
}

struct BuildEnvResult {
    env: Env,
    requested: Vec<String>,
    resolved: Vec<ResolvedPackageSnapshot>,
}

#[derive(Debug, Clone)]
struct BuildEnvScriptMeta {
    build_path: PathBuf,
    variant_index: Option<usize>,
    install: bool,
    install_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct ParseBuildArgsResult {
    env: HashMap<String, String>,
    remaining: Vec<String>,
}

/// Build a package from a `package.py` file and optional install target.
///
/// This resolves build-time requirements, constructs a build environment,
/// executes the selected build system, and optionally installs the resulting
/// package to a repository path.
pub fn build_package(
    package: &Package,
    package_path: &Path,
    storage: &Storage,
    options: &BuildOptions,
) -> Result<BuildReport, BuildError> {
    let source_dir = package_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let build_root = resolve_build_root(source_dir, package.build_directory.as_deref());

    let package_install_root = if options.install {
        Some(resolve_install_path(
            storage,
            options.prefix.as_ref(),
            package,
            options.build_type,
        )?)
    } else {
        None
    };

    let registry = BuildSystemRegistry::new();
    let build_system = resolve_build_system(
        package,
        source_dir,
        options.build_system.as_deref(),
        &registry,
    )?;

    let parse_result = parse_build_args(source_dir, &options.extra_args)?;
    let mut merged_build_args = options.build_args.clone();
    merged_build_args.extend(parse_result.remaining);

    let all_variants = collect_variants(package)?;
    let selected_variants = select_variants(&all_variants, &options.variants)?;

    let mut built_variants = 0;

    for variant in selected_variants {
        let variant_build_path = variant_build_path(&build_root, &variant);
        let variant_install_path = package_install_root
            .as_ref()
            .map(|root| variant_install_path(root, &variant));

        if options.clean {
            if variant_build_path.exists() {
                std::fs::remove_dir_all(&variant_build_path)?;
            }
            if let Some(path) = &variant_install_path {
                if path.exists() {
                    std::fs::remove_dir_all(path)?;
                }
            }
        }

        std::fs::create_dir_all(&variant_build_path)?;
        if let Some(path) = &variant_install_path {
            std::fs::create_dir_all(path)?;
        }

        if options.install && package.hashed_variants {
            if let (Some(package_root), Some(variant_root)) =
                (&package_install_root, &variant_install_path)
            {
                if let Err(err) = create_variant_shortlink(package_root, variant_root) {
                    eprintln!("Warning: variant shortlink not created: {}", err);
                }
            }
        }

        let rxt_path = variant_build_path.join("build.rxt");

        let BuildEnvResult {
            mut env,
            requested,
            resolved,
        } = create_build_env(
            package,
            storage,
            &variant,
            &variant_build_path,
            variant_install_path.as_ref(),
            package_path,
            &rxt_path,
            options.build_type,
        )?;

        apply_pre_build_commands(
            &mut env,
            package,
            &variant,
            source_dir,
            &variant_build_path,
            variant_install_path.as_ref(),
            options.build_type,
        )?;

        let solved_env = env
            .solve_impl(10, true)
            .unwrap_or_else(|_| env.compress());
        let mut env_map = solved_env.to_map();
        match ensure_msvc_env(&mut env_map) {
            MsvcEnvState::Applied(report) => {
                eprintln!(
                    "Info: MSVC env applied (VS {}, tools {}, SDK {:?}, UCRT {:?}, host {}, target {})",
                    report.vs_version,
                    report.tools_version,
                    report.sdk_version,
                    report.ucrt_version,
                    report.host,
                    report.target
                );
            }
            MsvcEnvState::Failed(err) => {
                return Err(BuildError::Config(format!(
                    "MSVC env bootstrap failed: {}",
                    err
                )));
            }
            MsvcEnvState::Skipped => {}
        }
        let script_env = Env::from_evars(
            solved_env.name.clone(),
            env_map
                .iter()
                .map(|(k, v)| Evar::set(k.clone(), v.clone())),
        );

        write_build_rxt(
            &rxt_path,
            storage,
            &requested,
            &resolved,
            &env_map,
        )?;
        write_variant_marker(&variant, &variant_build_path)?;

        if options.scripts {
            write_build_scripts(
                &variant_build_path,
                &script_env,
                &BuildEnvScriptMeta {
                    build_path: variant_build_path.clone(),
                    variant_index: variant.index,
                    install: options.install,
                    install_path: variant_install_path.clone(),
                },
            )?;
            continue;
        }

        let mut run_env = env_map.clone();
        for (key, value) in &parse_result.env {
            run_env.insert(key.clone(), value.clone());
        }

        let ctx = BuildContext {
            package,
            source_dir,
            build_dir: &variant_build_path,
            install_dir: variant_install_path.as_ref(),
            env: &run_env,
            variant_index: variant.index,
            install: options.install,
        };
        let args = BuildSystemArgs {
            build_args: &merged_build_args,
            child_build_args: &options.child_build_args,
        };
        build_system.before_phase(BuildPhase::Configure, &ctx, &args)?;
        build_system.configure(&ctx, &args)?;
        build_system.after_phase(BuildPhase::Configure, &ctx, &args)?;

        build_system.before_phase(BuildPhase::Build, &ctx, &args)?;
        build_system.build(&ctx, &args)?;
        build_system.after_phase(BuildPhase::Build, &ctx, &args)?;

        if options.install && build_system.supports_install() {
            build_system.before_phase(BuildPhase::Install, &ctx, &args)?;
            build_system.install(&ctx, &args)?;
            build_system.after_phase(BuildPhase::Install, &ctx, &args)?;
        }

        if let Some(path) = &package_install_root {
            install_package_files(package, package_path, path)?;
        }

        if let Some(install_path) = &variant_install_path {
            install_variant_metadata(&variant_build_path, install_path)?;
        }

        built_variants += 1;
    }

    Ok(BuildReport {
        built_variants,
        build_root,
        install_path: package_install_root,
    })
}

fn collect_variants(package: &Package) -> Result<Vec<BuildVariant>, BuildError> {
    if package.variants.is_empty() {
        return Ok(vec![BuildVariant {
            index: None,
            requires: Vec::new(),
            subpath: None,
        }]);
    }

    let mut variants = Vec::new();
    for (index, reqs) in package.variants.iter().enumerate() {
        let subpath = compute_variant_subpath(package, reqs);
        variants.push(BuildVariant {
            index: Some(index),
            requires: reqs.clone(),
            subpath,
        });
    }

    Ok(variants)
}

fn select_variants(
    all_variants: &[BuildVariant],
    requested: &[usize],
) -> Result<Vec<BuildVariant>, BuildError> {
    if requested.is_empty() {
        return Ok(all_variants.to_vec());
    }

    let mut selected = Vec::new();

    if all_variants.len() == 1 && all_variants[0].index.is_none() {
        if requested.iter().all(|v| *v == 0) {
            return Ok(all_variants.to_vec());
        }
        return Err(BuildError::Config(
            "package has no variants; only variant 0 is valid".to_string(),
        ));
    }

    let valid_indices: HashSet<usize> = all_variants
        .iter()
        .filter_map(|v| v.index)
        .collect();

    for index in requested {
        if !valid_indices.contains(index) {
            return Err(BuildError::Config(format!(
                "variant index {} is not valid for this package",
                index
            )));
        }
        if let Some(variant) = all_variants.iter().find(|v| v.index == Some(*index)) {
            selected.push(variant.clone());
        }
    }

    Ok(selected)
}

fn compute_variant_subpath(package: &Package, requires: &[String]) -> Option<String> {
    if package.hashed_variants {
        let list_repr = python_list_repr(requires);
        let mut hasher = Sha1::new();
        hasher.update(list_repr.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        return Some(hash);
    }

    if requires.is_empty() {
        return None;
    }

    let mut path = PathBuf::new();
    for req in requires {
        path.push(req);
    }
    Some(path.to_string_lossy().to_string())
}

fn python_list_repr(items: &[String]) -> String {
    let mut out = String::from("[");
    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push('\'');
        for ch in item.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '\'' => out.push_str("\\'"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
                other => out.push(other),
            }
        }
        out.push('\'');
    }
    out.push(']');
    out
}

fn variant_build_path(build_root: &Path, variant: &BuildVariant) -> PathBuf {
    match &variant.subpath {
        Some(subpath) if !subpath.is_empty() => build_root.join(subpath),
        _ => build_root.to_path_buf(),
    }
}

fn variant_install_path(package_root: &Path, variant: &BuildVariant) -> PathBuf {
    match &variant.subpath {
        Some(subpath) if !subpath.is_empty() => package_root.join(subpath),
        _ => package_root.to_path_buf(),
    }
}

fn resolve_build_root(source_dir: &Path, build_directory: Option<&str>) -> PathBuf {
    let build_dir = build_directory.unwrap_or("build");
    let build_path = PathBuf::from(build_dir);
    if build_path.is_absolute() {
        build_path
    } else {
        source_dir.join(build_path)
    }
}

fn resolve_install_path(
    storage: &Storage,
    prefix: Option<&PathBuf>,
    package: &Package,
    build_type: BuildType,
) -> Result<PathBuf, BuildError> {
    let base_config = crate::config::get().ok().cloned();
    let config = if let (Some(base), Some(override_value)) =
        (base_config.as_ref(), package.config.as_ref())
    {
        Some(
            crate::config::apply_package_override(base, override_value)
                .map_err(|e| BuildError::Config(e.to_string()))?,
        )
    } else {
        base_config
    };
    let config_paths = config
        .as_ref()
        .map(|cfg| crate::config::packages_path(cfg))
        .unwrap_or_default();
    let release_path = config
        .as_ref()
        .and_then(|cfg| crate::config::release_packages_path(cfg));
    let local_path = config
        .as_ref()
        .and_then(|cfg| crate::config::local_packages_path(cfg));
    let repo_root = if let Some(prefix) = prefix {
        prefix.clone()
    } else if build_type == BuildType::Central {
        if let Some(path) = release_path {
            path
        } else if let Some(first) = config_paths.first() {
            first.clone()
        } else if let Some(first) = storage.location_paths().first() {
            first.clone()
        } else {
            return Err(BuildError::Config(
                "no repository path available; use --prefix".to_string(),
            ));
        }
    } else if let Some(path) = local_path {
        path
    } else if let Some(first) = config_paths.first() {
        first.clone()
    } else {
        return Err(BuildError::Config(
            "no repository path available; use --prefix".to_string(),
        ));
    };

    Ok(repo_root.join(&package.base).join(&package.version))
}

fn resolve_build_system<'a>(
    package: &Package,
    source_dir: &Path,
    override_name: Option<&str>,
    registry: &'a BuildSystemRegistry,
) -> Result<&'a dyn systems::BuildSystem, BuildError> {
    if let Some(name) = override_name {
        return registry
            .by_name(name)
            .ok_or_else(|| BuildError::Config(format!("unsupported build system: {}", name)));
    }

    if package.build_command.is_some() {
        return registry.by_name("custom").ok_or_else(|| {
            BuildError::Config("custom build system is not registered".to_string())
        });
    }

    if let Some(name) = package.build_system.as_deref() {
        return registry
            .by_name(name)
            .ok_or_else(|| BuildError::Config(format!("unsupported build system: {}", name)));
    }

    if let Some(system) = registry.detect(source_dir) {
        return Ok(system);
    }

    Err(BuildError::Config(
        "no build system specified or detected".to_string(),
    ))
}

fn create_build_env(
    package: &Package,
    storage: &Storage,
    variant: &BuildVariant,
    build_path: &Path,
    install_path: Option<&PathBuf>,
    package_path: &Path,
    rxt_path: &Path,
    build_type: BuildType,
) -> Result<BuildEnvResult, BuildError> {
    let mut requested = Vec::new();
    // Rez order: requires + variant requires, then build/private build requires.
    requested.extend(package.reqs.iter().cloned());
    requested.extend(variant.requires.iter().cloned());
    requested.extend(package.build_requires.iter().cloned());
    requested.extend(package.private_build_requires.iter().cloned());

    let requested = dedup_preserve_order(requested);

    let mut build_pkg = Package::new("_build".to_string(), "0.0.0".to_string());
    build_pkg.reqs = requested.clone();

    if !build_pkg.reqs.is_empty() {
        build_pkg
            .solve(storage.packages())
            .map_err(|e| BuildError::Resolve(e.to_string()))?;
    }

    let mut env = build_pkg
        ._env("default", true)
        .unwrap_or_else(|| Env::new("build".to_string()));

    let build_vars = build_env_vars(
        package,
        &build_pkg,
        variant,
        build_path,
        install_path,
        package_path,
        &requested,
        rxt_path,
        build_type,
    );
    for (key, value) in build_vars {
        env.add(Evar::set(key, value));
    }

    let resolved = build_pkg
        .deps
        .iter()
        .map(|d| ResolvedPackageSnapshot {
            name: d.name.clone(),
            base: d.base.clone(),
            version: d.version.clone(),
        })
        .collect::<Vec<_>>();

    Ok(BuildEnvResult {
        env,
        requested,
        resolved,
    })
}

fn build_env_vars(
    package: &Package,
    _build_pkg: &Package,
    variant: &BuildVariant,
    build_path: &Path,
    install_path: Option<&PathBuf>,
    package_path: &Path,
    requested: &[String],
    rxt_path: &Path,
    build_type: BuildType,
) -> HashMap<String, String> {
    let thread_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let build_path_abs = build_path
        .canonicalize()
        .unwrap_or_else(|_| build_path.to_path_buf());
    let rxt_path_abs = build_path_abs.join(
        rxt_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("build.rxt")),
    );

    let package_file = package_path
        .canonicalize()
        .unwrap_or_else(|_| package_path.to_path_buf());
    let source_path = package_file
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let variant_index = variant.index.unwrap_or(0);
    let variant_requires = variant.requires.join(" ");
    let variant_subpath = variant.subpath.clone().unwrap_or_default();

    let requested_unversioned = requested
        .iter()
        .filter_map(|req| DepSpec::parse_impl(req).ok().map(|d| d.base))
        .collect::<Vec<_>>()
        .join(" ");

    let mut vars = HashMap::new();
    vars.insert("REZ_BUILD_ENV".to_string(), "1".to_string());
    vars.insert(
        "REZ_BUILD_PATH".to_string(),
        normalize_path_for_shell(&build_path_abs),
    );
    vars.insert(
        "REZ_BUILD_THREAD_COUNT".to_string(),
        thread_count.to_string(),
    );
    vars.insert(
        "REZ_BUILD_VARIANT_INDEX".to_string(),
        variant_index.to_string(),
    );
    vars.insert(
        "REZ_BUILD_VARIANT_REQUIRES".to_string(),
        variant_requires,
    );
    vars.insert(
        "REZ_BUILD_VARIANT_SUBPATH".to_string(),
        variant_subpath,
    );
    vars.insert(
        "REZ_BUILD_PROJECT_VERSION".to_string(),
        package.version.clone(),
    );
    vars.insert(
        "REZ_BUILD_PROJECT_NAME".to_string(),
        package.base.clone(),
    );
    vars.insert(
        "REZ_BUILD_PROJECT_DESCRIPTION".to_string(),
        package
            .description
            .clone()
            .unwrap_or_default()
            .trim()
            .to_string(),
    );
    vars.insert(
        "REZ_BUILD_PROJECT_FILE".to_string(),
        normalize_path_for_shell(&package_file),
    );
    vars.insert(
        "REZ_BUILD_SOURCE_PATH".to_string(),
        normalize_path_for_shell(source_path),
    );
    vars.insert(
        "REZ_BUILD_REQUIRES".to_string(),
        requested.join(" "),
    );
    vars.insert(
        "REZ_BUILD_REQUIRES_UNVERSIONED".to_string(),
        requested_unversioned,
    );
    vars.insert(
        "REZ_BUILD_TYPE".to_string(),
        build_type.as_str().to_string(),
    );
    vars.insert(
        "REZ_BUILD_INSTALL".to_string(),
        if install_path.is_some() { "1" } else { "0" }.to_string(),
    );
    vars.insert(
        "REZ_RXT_FILE".to_string(),
        normalize_path_for_shell(&rxt_path_abs),
    );

    if let Some(path) = install_path {
        vars.insert(
            "REZ_BUILD_INSTALL_PATH".to_string(),
            normalize_path_for_shell(path),
        );
    }

    if build_type == BuildType::Central {
        vars.insert("REZ_IN_REZ_RELEASE".to_string(), "1".to_string());
    }

    vars
}

pub(crate) fn normalize_path_for_shell(path: &Path) -> String {
    let mut s = path.display().to_string();
    if cfg!(windows) {
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            s = format!(r"\\{}", rest);
        } else if let Some(rest) = s.strip_prefix(r"\\?\") {
            s = rest.to_string();
        }
    }
    s
}

fn write_build_rxt(
    rxt_path: &Path,
    storage: &Storage,
    requested: &[String],
    resolved: &[ResolvedPackageSnapshot],
    env: &HashMap<String, String>,
) -> Result<(), BuildError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    let resolved_packages = resolved
        .iter()
        .map(|pkg| build_variant_handle(storage, pkg))
        .collect::<Vec<_>>();

    let rez_path = std::env::current_exe()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "pkg".to_string());

    let snapshot = ResolvedContextSnapshot {
        serialize_version: "4.9".to_string(),
        timestamp,
        requested_timestamp: timestamp,
        building: true,
        testing: false,
        caching: false,
        implicit_packages: Vec::new(),
        package_requests: requested.to_vec(),
        package_paths: storage
            .location_paths()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        append_sys_path: false,
        package_caching: false,
        package_cache_async: false,
        default_patch_lock: "no_lock".to_string(),
        patch_locks: HashMap::new(),
        package_orderers: None,
        package_filter: Vec::new(),
        graph: "{}".to_string(),
        resolved_packages,
        resolved_ephemerals: Vec::new(),
        rez_version: crate::VERSION.to_string(),
        rez_path,
        user,
        host,
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        os: std::env::consts::OS.to_string(),
        created: timestamp,
        parent_suite_path: None,
        suite_context_name: None,
        status: "solved".to_string(),
        failure_description: None,
        from_cache: false,
        solve_time: 0.0,
        load_time: 0.0,
        num_loaded_packages: resolved.len() as i64,
        pkg_env: env.clone(),
    };

    let content = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| BuildError::Config(e.to_string()))?;
    std::fs::write(rxt_path, content)?;
    Ok(())
}

fn build_variant_handle(
    storage: &Storage,
    pkg: &ResolvedPackageSnapshot,
) -> ResourceHandleSnapshot {
    let mut variables = HashMap::new();
    variables.insert(
        "repository_type".to_string(),
        serde_json::Value::String("filesystem".to_string()),
    );

    let location = find_package_location(storage, pkg)
        .or_else(|| storage.location_paths().first().cloned())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    variables.insert("location".to_string(), serde_json::Value::String(location));
    variables.insert(
        "name".to_string(),
        serde_json::Value::String(pkg.base.clone()),
    );
    variables.insert(
        "version".to_string(),
        serde_json::Value::String(pkg.version.clone()),
    );

    ResourceHandleSnapshot {
        key: "filesystem.variant".to_string(),
        variables,
    }
}

fn find_package_location(
    storage: &Storage,
    pkg: &ResolvedPackageSnapshot,
) -> Option<PathBuf> {
    let name = format!("{}-{}", pkg.base, pkg.version);
    let package = storage.get(&name)?;
    let source = package.package_source?;
    let path = Path::new(&source);
    let version_dir = path.parent()?;
    let base_dir = version_dir.parent()?;
    let repo_dir = base_dir.parent()?;
    Some(repo_dir.to_path_buf())
}

fn write_variant_marker(variant: &BuildVariant, build_root: &Path) -> Result<(), BuildError> {
    let Some(index) = variant.index else {
        return Ok(());
    };

    let data = serde_json::json!({
        "index": index,
        "data": variant.requires.clone(),
    });
    let content = serde_json::to_string_pretty(&data)
        .map_err(|e| BuildError::Config(e.to_string()))?;
    std::fs::write(build_root.join("variant.json"), content)?;
    Ok(())
}

fn create_variant_shortlink(
    package_root: &Path,
    variant_root: &Path,
) -> Result<(), BuildError> {
    if !use_variant_shortlinks() {
        return Ok(());
    }

    let dirname = variant_shortlinks_dirname();
    let shortlinks_dir = package_root.join(dirname);
    std::fs::create_dir_all(&shortlinks_dir)?;

    let target = variant_root
        .canonicalize()
        .unwrap_or_else(|_| variant_root.to_path_buf());

    create_unique_base26_symlink(&shortlinks_dir, &target)?;
    Ok(())
}

fn use_variant_shortlinks() -> bool {
    std::env::var("PKG_USE_VARIANT_SHORTLINKS")
        .ok()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn variant_shortlinks_dirname() -> String {
    std::env::var("PKG_VARIANT_SHORTLINKS_DIRNAME").unwrap_or_else(|_| "_v".to_string())
}

fn create_unique_base26_symlink(path: &Path, target: &Path) -> Result<PathBuf, BuildError> {
    if let Some(existing) = find_matching_symlink(path, target) {
        return Ok(existing);
    }

    let mut retries = 0;
    loop {
        let prev = find_max_symlink_name(path);
        let linkname = get_next_base26(prev.as_deref())?;
        let linkpath = path.join(&linkname);

        match create_symlink_dir(target, &linkpath) {
            Ok(()) => return Ok(linkpath),
            Err(e) => {
                if retries > 10 {
                    return Err(BuildError::Config(format!(
                        "variant shortlink not created: {}",
                        e
                    )));
                }
                retries += 1;
                continue;
            }
        }
    }
}

fn create_symlink_dir(target: &Path, linkpath: &Path) -> Result<(), std::io::Error> {
    if linkpath.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, linkpath)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, linkpath)
    }
}

fn find_matching_symlink(path: &Path, target: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let file_type = entry.file_type().ok()?;
        if !file_type.is_symlink() {
            continue;
        }
        let link_path = entry.path();
        let link_target = std::fs::read_link(&link_path).ok()?;
        let resolved = if link_target.is_absolute() {
            link_target
        } else {
            link_path.parent()?.join(link_target)
        };
        if resolved == target {
            return Some(link_path);
        }
    }
    None
}

fn find_max_symlink_name(path: &Path) -> Option<String> {
    let entries = std::fs::read_dir(path).ok()?;
    let mut max_name: Option<String> = None;
    for entry in entries.flatten() {
        let file_type = entry.file_type().ok()?;
        if !file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.chars().all(|c| c.is_ascii_lowercase()) {
            continue;
        }
        if max_name.as_ref().map_or(true, |m| name > *m) {
            max_name = Some(name);
        }
    }
    max_name
}

fn get_next_base26(prev: Option<&str>) -> Result<String, BuildError> {
    if prev.is_none() {
        return Ok("a".to_string());
    }
    let prev = prev.unwrap();
    if !prev.chars().all(|c| c.is_ascii_lowercase()) {
        return Err(BuildError::Config("invalid base26 id".to_string()));
    }
    if !prev.ends_with('z') {
        let mut chars: Vec<char> = prev.chars().collect();
        if let Some(last) = chars.pop() {
            let next = ((last as u8) + 1) as char;
            chars.push(next);
        }
        return Ok(chars.into_iter().collect());
    }
    let prefix = &prev[..prev.len() - 1];
    Ok(format!("{}a", get_next_base26(Some(prefix))?))
}

fn write_build_scripts(
    build_root: &Path,
    env: &Env,
    meta: &BuildEnvScriptMeta,
) -> Result<(), BuildError> {
    let cmd_path = build_root.join("build_env.cmd");
    let ps1_path = build_root.join("build_env.ps1");
    let sh_path = build_root.join("build_env.sh");
    let forward_path = build_root.join("build-env");

    let cmd_body = format!(
        "@echo off\r\ncd /d \"{}\"\r\n{}\r\n",
        build_root.display(),
        env.to_cmd()
    );
    let ps1_body = format!(
        "Set-Location -Path \"{}\"\n{}\n",
        build_root.display(),
        env.to_ps1()
    );
    let sh_body = format!(
        "#!/usr/bin/env bash\ncd \"{}\"\n{}\n",
        build_root.display(),
        env.to_sh()
    );

    std::fs::write(cmd_path, cmd_body)?;
    std::fs::write(ps1_path, ps1_body)?;
    std::fs::write(sh_path, sh_body)?;

    write_build_env_forwarder(&forward_path, meta)?;

    Ok(())
}

fn write_build_env_forwarder(
    path: &Path,
    meta: &BuildEnvScriptMeta,
) -> Result<(), BuildError> {
    let exe = std::env::current_exe().map_err(BuildError::Io)?;
    let exe_str = exe.display().to_string();

    let mut args = Vec::new();
    args.push("build-env".to_string());
    args.push("--build-path".to_string());
    args.push(meta.build_path.display().to_string());

    if let Some(index) = meta.variant_index {
        args.push("--variant-index".to_string());
        args.push(index.to_string());
    }

    if meta.install {
        args.push("--install".to_string());
    }

    if let Some(install_path) = &meta.install_path {
        args.push("--install-path".to_string());
        args.push(install_path.display().to_string());
    }

    if cfg!(windows) {
        let cmd_path = if path.extension().and_then(|s| s.to_str()) == Some("cmd") {
            path.to_path_buf()
        } else {
            path.with_extension("cmd")
        };
        let cmd_line = std::iter::once(shell_quote(&exe_str))
            .chain(args.iter().map(|a| shell_quote(a)))
            .collect::<Vec<_>>()
            .join(" ");
        let body = format!("@echo off\r\n{}\r\n", cmd_line);
        std::fs::write(cmd_path, body)?;
    } else {
        let cmd_line = std::iter::once(shell_quote(&exe_str))
            .chain(args.iter().map(|a| shell_quote(a)))
            .collect::<Vec<_>>()
            .join(" ");
        let body = format!("#!/usr/bin/env bash\n{}\n", cmd_line);
        std::fs::write(path, body)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms)?;
        }
    }

    Ok(())
}

fn run_command(
    program: &str,
    args: &[String],
    working_dir: &Path,
    env: &HashMap<String, String>,
) -> Result<(), BuildError> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.current_dir(working_dir);
    cmd.envs(env);

    let status = cmd.status().map_err(BuildError::Io)?;
    if status.success() {
        Ok(())
    } else {
        Err(BuildError::CommandFailed {
            command: format!("{} {}", program, args.join(" ")),
            code: status.code(),
        })
    }
}

fn run_shell_command(
    command: &str,
    working_dir: &Path,
    env: &HashMap<String, String>,
) -> Result<(), BuildError> {
    let mut cmd = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", command]);
        cmd
    };

    cmd.current_dir(working_dir);
    cmd.envs(env);

    let status = cmd.status().map_err(BuildError::Io)?;
    if status.success() {
        Ok(())
    } else {
        Err(BuildError::CommandFailed {
            command: command.to_string(),
            code: status.code(),
        })
    }
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }
    if cfg!(windows) {
        let escaped = value.replace('"', "\"\"");
        if escaped.contains(' ') || escaped.contains('\t') {
            format!("\"{}\"", escaped)
        } else {
            escaped
        }
    } else {
        if value.chars().all(|c| c.is_ascii_alphanumeric() || "-._/".contains(c)) {
            value.to_string()
        } else {
            format!("'{}'", value.replace('\'', "'\\''"))
        }
    }
}

fn install_package_files(
    package: &Package,
    package_path: &Path,
    install_path: &Path,
) -> Result<(), BuildError> {
    let dest = install_path.join("package.py");
    std::fs::create_dir_all(install_path)?;
    std::fs::copy(package_path, dest)?;

    if package.package_source.is_none() {
        return Ok(());
    }

    Ok(())
}

fn install_variant_metadata(build_root: &Path, install_path: &Path) -> Result<(), BuildError> {
    let rxt_src = build_root.join("build.rxt");
    if rxt_src.exists() {
        let dest = install_path.join("build.rxt");
        std::fs::copy(&rxt_src, dest)?;
    }

    let variant_src = build_root.join("variant.json");
    if variant_src.exists() {
        let dest = install_path.join("variant.json");
        std::fs::copy(&variant_src, dest)?;
    }

    let forward_src = build_root.join("build-env");
    if forward_src.exists() {
        let dest = install_path.join("build-env");
        std::fs::copy(&forward_src, dest)?;
    }
    let forward_cmd = build_root.join("build-env.cmd");
    if forward_cmd.exists() {
        let dest = install_path.join("build-env.cmd");
        std::fs::copy(&forward_cmd, dest)?;
    }

    Ok(())
}

fn parse_build_args(
    source_dir: &Path,
    extra_args: &[String],
) -> Result<ParseBuildArgsResult, BuildError> {
    let path = source_dir.join("parse_build_args.py");
    if !path.exists() {
        return Ok(ParseBuildArgsResult {
            env: HashMap::new(),
            remaining: extra_args.to_vec(),
        });
    }

    let source = std::fs::read_to_string(&path)?;
    if source.trim().is_empty() {
        return Ok(ParseBuildArgsResult {
            env: HashMap::new(),
            remaining: extra_args.to_vec(),
        });
    }

    let _ = Python::initialize();
    Python::attach(|py| {
        let argparse = py.import("argparse").map_err(|e| {
            BuildError::Config(format!("parse_build_args import failed: {e}"))
        })?;
        let parser = argparse
            .getattr("ArgumentParser")
            .map_err(|e| BuildError::Config(format!("parse_build_args parser failed: {e}")))?;
        let parser = parser
            .call1(())
            .map_err(|e| BuildError::Config(format!("parse_build_args parser init failed: {e}")))?;

        let globals = PyDict::new(py);
        globals.set_item("parser", &parser).ok();

        let source = std::ffi::CString::new(source).map_err(|e| {
            BuildError::Config(format!("parse_build_args exec failed: {e}"))
        })?;
        if let Err(err) = py.run(source.as_c_str(), Some(&globals), None) {
            eprintln!("Warning: parse_build_args.py failed: {err}");
            return Ok(ParseBuildArgsResult {
                env: HashMap::new(),
                remaining: extra_args.to_vec(),
            });
        }

        let args_list = PyList::new(py, extra_args).map_err(|e| {
            BuildError::Config(format!("parse_build_args args failed: {e}"))
        })?;
        let parsed = parser
            .getattr("parse_known_args")
            .map_err(|e| BuildError::Config(format!("parse_build_args parse failed: {e}")))?
            .call1((args_list,))
            .map_err(|e| BuildError::Config(format!("parse_build_args parse failed: {e}")))?;

        let tuple: (pyo3::Py<pyo3::PyAny>, pyo3::Py<pyo3::PyAny>) = parsed.extract().map_err(|e| {
            BuildError::Config(format!("parse_build_args parse failed: {e}"))
        })?;

        let ns = tuple.0.bind(py);
        let unknown = tuple.1.bind(py);

        let vars_func = py
            .import("builtins")
            .and_then(|b| b.getattr("vars"))
            .map_err(|e| BuildError::Config(format!("parse_build_args vars failed: {e}")))?;
        let ns_any = vars_func
            .call1((ns,))
            .map_err(|e| BuildError::Config(format!("parse_build_args vars failed: {e}")))?;
        let ns_dict = ns_any
            .cast::<PyDict>()
            .map_err(|e| BuildError::Config(format!("parse_build_args vars failed: {e}")))?;

        let shlex = py.import("shlex").map_err(|e| {
            BuildError::Config(format!("parse_build_args shlex failed: {e}"))
        })?;
        let quote = shlex
            .getattr("quote")
            .map_err(|e| BuildError::Config(format!("parse_build_args shlex failed: {e}")))?;

        let mut env = HashMap::new();
        for (key, value) in ns_dict.iter() {
            let key: String = key.extract().map_err(|e| {
                BuildError::Config(format!("parse_build_args key failed: {e}"))
            })?;
            if value.is_none() {
                continue;
            }
            let env_key = format!("__PARSE_ARG_{}", key.to_ascii_uppercase());
            let val = if value.is_instance_of::<PyBool>() {
                let flag = value.extract::<bool>().map_err(|e| {
                    BuildError::Config(format!("parse_build_args bool failed: {e}"))
                })?;
                if flag {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            } else if value.is_instance_of::<PyList>() || value.is_instance_of::<PyTuple>() {
                let items: Vec<String> = value.extract().map_err(|e| {
                    BuildError::Config(format!("parse_build_args list failed: {e}"))
                })?;
                let mut quoted = Vec::new();
                for item in items {
                    let q = quote.call1((item,)).map_err(|e| {
                        BuildError::Config(format!("parse_build_args list failed: {e}"))
                    })?;
                    let q: String = q.extract().map_err(|e| {
                        BuildError::Config(format!("parse_build_args list failed: {e}"))
                    })?;
                    quoted.push(q);
                }
                quoted.join(" ")
            } else {
                value.extract::<String>().map_err(|e| {
                    BuildError::Config(format!("parse_build_args value failed: {e}"))
                })?
            };
            env.insert(env_key, val);
        }

        let remaining: Vec<String> = unknown.extract().map_err(|e| {
            BuildError::Config(format!("parse_build_args unknown failed: {e}"))
        })?;

        Ok(ParseBuildArgsResult { env, remaining })
    })
}

fn apply_pre_build_commands(
    env: &mut Env,
    package: &Package,
    variant: &BuildVariant,
    source_dir: &Path,
    build_path: &Path,
    install_path: Option<&PathBuf>,
    build_type: BuildType,
) -> Result<(), BuildError> {
    let Some(source) = package.pre_build_commands.as_deref() else {
        return Ok(());
    };

    let source_dir_abs = source_dir
        .canonicalize()
        .unwrap_or_else(|_| source_dir.to_path_buf());
    let build_path_abs = build_path
        .canonicalize()
        .unwrap_or_else(|_| build_path.to_path_buf());
    let install_path_abs =
        install_path.map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()));

    let _ = Python::initialize();

    Python::attach(|py| {
        let globals = PyDict::new(py);
        if let Ok(os_mod) = py.import("os") {
            globals.set_item("os", os_mod).ok();
        }
        if let Ok(sys_mod) = py.import("sys") {
            globals.set_item("sys", sys_mod).ok();
        }

        let bootstrap = r#"
class _EnvVarProxy:
    def __init__(self, env, name):
        self._env = env
        self._name = name

    def append(self, value):
        self._env._evars.append(("append", self._name, value))

    def prepend(self, value):
        self._env._evars.append(("insert", self._name, value))

    def set(self, value):
        self._env._evars.append(("set", self._name, value))

class _BuildEnv:
    def __init__(self):
        self._evars = []

    def __getattr__(self, name):
        return _EnvVarProxy(self, name)

    def __setattr__(self, name, value):
        if name.startswith("_"):
            object.__setattr__(self, name, value)
        else:
            self._evars.append(("set", name, value))

env = _BuildEnv()

class _ROAttrDictWrapper:
    def __init__(self, data):
        object.__setattr__(self, "_data", data)

    def __getattr__(self, attr):
        data = object.__getattribute__(self, "_data")
        try:
            return data[attr]
        except KeyError:
            raise AttributeError("'%s' object has no attribute '%s'" % (self.__class__.__name__, attr))

    def __setattr__(self, attr, value):
        if attr.startswith("__") and attr.endswith("__"):
            object.__setattr__(self, attr, value)
            return
        if attr in object.__getattribute__(self, "_data"):
            raise AttributeError("'%s' object attribute '%s' is read-only" % (self.__class__.__name__, attr))
        raise AttributeError("'%s' object has no attribute '%s'" % (self.__class__.__name__, attr))

    def __getitem__(self, key):
        return object.__getattribute__(self, "_data")[key]

    def __contains__(self, key):
        return key in object.__getattribute__(self, "_data")

    def __iter__(self):
        return iter(object.__getattribute__(self, "_data"))

    def __len__(self):
        return len(object.__getattribute__(self, "_data"))

    def __repr__(self):
        return "%s(%r)" % (self.__class__.__name__, object.__getattribute__(self, "_data"))

class _VersionBinding:
    def __init__(self, value):
        self._value = str(value)
        self._parts = [p for p in self._value.split(".") if p]

    @property
    def major(self):
        return self[0]

    @property
    def minor(self):
        return self[1]

    @property
    def patch(self):
        return self[2]

    def __getitem__(self, idx):
        try:
            part = self._parts[idx]
        except IndexError:
            return None
        try:
            return int(part)
        except ValueError:
            return part

    def __len__(self):
        return len(self._parts)

    def __iter__(self):
        return iter(self._parts)

    def __str__(self):
        return self._value

class _VariantBinding:
    def __init__(self, data):
        object.__setattr__(self, "_data", data)
        object.__setattr__(self, "version", _VersionBinding(data.get("version", "")))

    def __getattr__(self, attr):
        data = object.__getattribute__(self, "_data")
        if attr in data:
            return data[attr]
        raise AttributeError("package %s has no attribute '%s'" % (self.__str__(), attr))

    def __str__(self):
        data = object.__getattribute__(self, "_data")
        return data.get("qualified_package_name") or data.get("name", "")
"#;

        let bootstrap = std::ffi::CString::new(bootstrap).map_err(|e| {
            BuildError::Config(format!("pre_build_commands bootstrap failed: {e}"))
        })?;
        py.run(bootstrap.as_c_str(), Some(&globals), None).map_err(|e| {
            BuildError::Config(format!("pre_build_commands bootstrap failed: {e}"))
        })?;

        let this_kwargs = PyDict::new(py);
        this_kwargs
            .set_item("root", normalize_path_for_shell(&source_dir_abs))
            .ok();
        this_kwargs
            .set_item("name", package.base.clone())
            .ok();
        this_kwargs
            .set_item("version", package.version.clone())
            .ok();
        this_kwargs
            .set_item(
                "qualified_package_name",
                format!("{}-{}", package.base, package.version),
            )
            .ok();
        this_kwargs
            .set_item(
                "variant_index",
                variant.index.map(|v| v as i64),
            )
            .ok();
        this_kwargs
            .set_item("variant_requires", variant.requires.clone())
            .ok();
        this_kwargs
            .set_item("variant_subpath", variant.subpath.clone())
            .ok();
        this_kwargs
            .set_item("build_path", normalize_path_for_shell(&build_path_abs))
            .ok();
        this_kwargs
            .set_item(
                "install_path",
                install_path_abs
                    .as_ref()
                    .map(|p| normalize_path_for_shell(p)),
            )
            .ok();

        let variant_cls = globals
            .get_item("_VariantBinding")
            .map_err(|e| BuildError::Config(format!("pre_build_commands bind failed: {e}")))?
            .ok_or_else(|| BuildError::Config("pre_build_commands bind failed".to_string()))?;
        let this_obj = variant_cls
            .call1((this_kwargs,))
            .map_err(|e| BuildError::Config(format!("pre_build_commands this init failed: {e}")))?;
        globals.set_item("this", this_obj).ok();

        let build_kwargs = PyDict::new(py);
        build_kwargs
            .set_item("build_type", build_type.as_str())
            .ok();
        build_kwargs
            .set_item("install", install_path.is_some())
            .ok();
        build_kwargs
            .set_item("build_path", normalize_path_for_shell(&build_path_abs))
            .ok();
        build_kwargs
            .set_item(
                "install_path",
                install_path_abs
                    .as_ref()
                    .map(|p| normalize_path_for_shell(p)),
            )
            .ok();
        let ro_cls = globals
            .get_item("_ROAttrDictWrapper")
            .map_err(|e| BuildError::Config(format!("pre_build_commands bind failed: {e}")))?
            .ok_or_else(|| BuildError::Config("pre_build_commands bind failed".to_string()))?;
        let build_obj = ro_cls
            .call1((build_kwargs,))
            .map_err(|e| BuildError::Config(format!("pre_build_commands build init failed: {e}")))?;
        globals.set_item("build", build_obj).ok();

        let source = std::ffi::CString::new(source).map_err(|e| {
            BuildError::Config(format!("pre_build_commands exec failed: {e}"))
        })?;
        py.run(source.as_c_str(), Some(&globals), None).map_err(|e| {
            BuildError::Config(format!("pre_build_commands exec failed: {e}"))
        })?;

        if let Ok(Some(func)) = globals.get_item("pre_build_commands") {
            if func.is_callable() {
                func.call0().map_err(|e| {
                    BuildError::Config(format!("pre_build_commands call failed: {e}"))
                })?;
            }
        }

        let env_obj = globals
            .get_item("env")
            .map_err(|e| BuildError::Config(format!("pre_build_commands env failed: {e}")))?;
        let Some(env_obj) = env_obj else {
            return Ok(());
        };
        let evars = env_obj
            .getattr("_evars")
            .map_err(|e| BuildError::Config(format!("pre_build_commands env read failed: {e}")))?;
        let evars: Vec<(String, String, String)> = evars
            .extract()
            .map_err(|e| BuildError::Config(format!("pre_build_commands env parse failed: {e}")))?;

        for (action, name, value) in evars {
            let evar = match action.as_str() {
                "append" => Evar::append(name, value),
                "insert" => Evar::insert(name, value),
                "set" => Evar::set(name, value),
                other => {
                    return Err(BuildError::Config(format!(
                        "pre_build_commands unknown action: {}",
                        other
                    )))
                }
            };
            env.add(evar);
        }

        Ok(())
    })
}

fn dedup_preserve_order(items: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_list_repr_matches() {
        let items = vec!["python-3.10".to_string(), "foo".to_string()];
        let repr = python_list_repr(&items);
        assert_eq!(repr, "['python-3.10', 'foo']");
    }

    #[test]
    fn hashed_variant_subpath() {
        let mut pkg = Package::new("foo".to_string(), "1.0.0".to_string());
        pkg.hashed_variants = true;
        pkg.variants = vec![vec!["python@>=3.10".to_string()]];
        let variants = collect_variants(&pkg).unwrap();
        assert_eq!(variants.len(), 1);
        assert!(variants[0].subpath.as_ref().unwrap().len() >= 8);
    }
}
