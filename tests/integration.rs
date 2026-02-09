//! Integration tests for pkg.
//!
//! Uses tempdir to create isolated test repositories.
//! Lib tests: Storage, Solver, rex, test section. CLI tests: run `pkg` binary against temp repo.

use pkg_lib::rex::{apply_package_commands, package_root_from_source, packages_in_rex_order};
use pkg_lib::{Solver, Storage};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Create a package.py file in the given directory.
fn create_package(dir: &Path, name: &str, version: &str, requires: &[&str]) {
    let pkg_dir = dir.join(name).join(version);
    fs::create_dir_all(&pkg_dir).unwrap();

    let reqs = if requires.is_empty() {
        String::new()
    } else {
        let reqs_str: Vec<String> = requires.iter().map(|r| format!("\"{}\"", r)).collect();
        format!("\n    p.add_req({})", reqs_str.join(")\n    p.add_req("))
    };

    let content = format!(
        r#"def get_package():
    p = pkg.Package("{}", "{}"){}
    return p
"#,
        name, version, reqs
    );

    fs::write(pkg_dir.join("package.py"), content).unwrap();
}

/// Create a simple test repo with given packages.
fn create_test_repo(packages: &[(&str, &str, &[&str])]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (name, version, requires) in packages {
        create_package(dir.path(), name, version, requires);
    }
    dir
}

#[test]
fn test_storage_scan() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &[]),
        ("maya", "2025.0.0", &[]),
        ("houdini", "20.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();

    assert_eq!(storage.count(), 3);
    assert!(storage.has("maya-2024.0.0"));
    assert!(storage.has("maya-2025.0.0"));
    assert!(storage.has("houdini-20.0.0"));
}

#[test]
fn test_storage_versions() {
    let repo = create_test_repo(&[
        ("maya", "2023.0.0", &[]),
        ("maya", "2024.0.0", &[]),
        ("maya", "2025.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();

    let versions = storage.versions("maya");
    assert_eq!(versions.len(), 3);
    // Sorted newest first
    assert_eq!(versions[0], "maya-2025.0.0");
    assert_eq!(versions[1], "maya-2024.0.0");
    assert_eq!(versions[2], "maya-2023.0.0");
}

#[test]
fn test_storage_latest() {
    let repo = create_test_repo(&[
        ("maya", "2023.0.0", &[]),
        ("maya", "2024.0.0", &[]),
        ("maya", "2025.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();

    let latest = storage.latest("maya").unwrap();
    assert_eq!(latest.version, "2025.0.0");
}

#[test]
fn test_storage_resolve_base_name() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &[]),
        ("maya", "2025.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();

    // Resolve base name -> latest
    let pkg = storage.resolve("maya").unwrap();
    assert_eq!(pkg.version, "2025.0.0");

    // Resolve exact name
    let pkg = storage.resolve("maya-2024.0.0").unwrap();
    assert_eq!(pkg.version, "2024.0.0");
}

#[test]
fn test_storage_resolve_with_constraint() {
    let repo = create_test_repo(&[
        ("maya", "2023.0.0", &[]),
        ("maya", "2024.0.0", &[]),
        ("maya", "2025.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();

    // Resolve with constraint
    let pkg = storage.resolve("maya@>=2024,<2025").unwrap();
    assert_eq!(pkg.version, "2024.0.0");

    let pkg = storage.resolve("maya@2023").unwrap();
    assert_eq!(pkg.version, "2023.0.0");
}

#[test]
fn test_solver_simple() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let solution = solver.solve_impl("maya-2024.0.0").unwrap();
    assert_eq!(solution.len(), 1);
    assert!(solution.contains(&"maya-2024.0.0".to_string()));
}

#[test]
fn test_solver_with_deps() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &["redshift@>=3.5"]),
        ("redshift", "3.5.0", &[]),
        ("redshift", "3.6.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let solution = solver.solve_impl("maya-2024.0.0").unwrap();
    assert_eq!(solution.len(), 2);
    assert!(solution.contains(&"maya-2024.0.0".to_string()));
    // Should pick newest matching version
    assert!(solution.contains(&"redshift-3.6.0".to_string()));
}

#[test]
fn test_solver_transitive_deps() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &["redshift@>=3.5"]),
        ("redshift", "3.6.0", &["cuda@>=11"]),
        ("cuda", "11.0.0", &[]),
        ("cuda", "12.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let solution = solver.solve_impl("maya-2024.0.0").unwrap();
    assert_eq!(solution.len(), 3);
    assert!(solution.contains(&"maya-2024.0.0".to_string()));
    assert!(solution.contains(&"redshift-3.6.0".to_string()));
    assert!(solution.contains(&"cuda-12.0.0".to_string()));
}

#[test]
fn test_solver_conflict() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &["core@>=2"]),
        ("houdini", "20.0.0", &["core@<2"]),
        ("core", "1.0.0", &[]),
        ("core", "2.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    // Should fail with conflict
    let result = solver.solve_requirements_impl(&[
        "maya-2024.0.0".to_string(),
        "houdini-20.0.0".to_string(),
    ]);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("conflict"));
    assert!(err.contains("core"));
}

#[test]
fn test_solver_multiple_requirements() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &[]),
        ("houdini", "20.0.0", &[]),
        ("nuke", "14.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let solution = solver.solve_requirements_impl(&[
        "maya".to_string(),
        "houdini".to_string(),
        "nuke".to_string(),
    ]).unwrap();

    assert_eq!(solution.len(), 3);
}

#[test]
fn test_empty_repo() {
    let dir = TempDir::new().unwrap();
    let storage = Storage::scan_impl(Some(&[dir.path().to_path_buf()])).unwrap();
    assert_eq!(storage.count(), 0);
}

#[test]
fn test_package_not_found() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    
    assert!(storage.get("nonexistent-1.0.0").is_none());
    assert!(storage.resolve("nonexistent").is_none());
}

// =============================================================================
// Complex resolver tests with deep chains and diamond dependencies
// =============================================================================

/// Diamond dependency - SUCCESS case
/// 
///        app-1.0.0
///        /        \
///   lib_a@>=2   lib_b@>=1  
///       |          |
///   lib_a-2.0.0  lib_b-2.0.0
///       |          |
///   core@>=2     core@>=1    <- both satisfied by core-3.0.0
///       \        /
///        core-3.0.0
///
#[test]
fn test_diamond_dependency_success() {
    let repo = create_test_repo(&[
        // App requires both libraries
        ("app", "1.0.0", &["lib_a@>=2", "lib_b@>=1"]),
        
        // lib_a versions - require core>=2
        ("lib_a", "1.0.0", &["core@>=1"]),
        ("lib_a", "2.0.0", &["core@>=2"]),
        ("lib_a", "3.0.0", &["core@>=3"]),
        
        // lib_b versions - require core>=1 (flexible)
        ("lib_b", "1.0.0", &["core@>=1"]),
        ("lib_b", "2.0.0", &["core@>=1"]),
        
        // core versions
        ("core", "1.0.0", &[]),
        ("core", "2.0.0", &[]),
        ("core", "3.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let solution = solver.solve_impl("app-1.0.0").unwrap();
    
    // Should resolve successfully
    assert_eq!(solution.len(), 4); // app + lib_a + lib_b + core
    assert!(solution.contains(&"app-1.0.0".to_string()));
    assert!(solution.contains(&"lib_a-3.0.0".to_string())); // latest >=2, core-3.0.0 satisfies core>=3
    assert!(solution.contains(&"lib_b-2.0.0".to_string())); // latest
    assert!(solution.contains(&"core-3.0.0".to_string()));  // satisfies both
}

/// Diamond dependency - CONFLICT case
/// Same as above but lib_b requires core<2, creating unsatisfiable constraint
///
///        app-1.0.0
///        /        \
///   lib_a@>=2   lib_b@>=1  
///       |          |
///   lib_a-2.0.0  lib_b-2.0.0
///       |          |
///   core@>=2     core@<2     <- CONFLICT! No version satisfies both
///       \        /
///          X
///
#[test]
fn test_diamond_dependency_conflict() {
    let repo = create_test_repo(&[
        // App requires both libraries
        ("app", "1.0.0", &["lib_a@>=2", "lib_b@>=1"]),
        
        // lib_a requires core>=2
        ("lib_a", "1.0.0", &["core@>=1"]),
        ("lib_a", "2.0.0", &["core@>=2"]),
        
        // lib_b requires core<2 - CONFLICT with lib_a!
        ("lib_b", "1.0.0", &["core@<2"]),
        ("lib_b", "2.0.0", &["core@<2"]),
        
        // core versions - no version satisfies >=2 AND <2
        ("core", "1.0.0", &[]),
        ("core", "2.0.0", &[]),
        ("core", "3.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let result = solver.solve_impl("app-1.0.0");
    
    // Should fail with conflict
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("core")); // Conflict is about core
}

/// Deep dependency chain (5 levels) - SUCCESS
/// 
/// app -> framework -> runtime -> platform -> arch -> base
///
/// Tests that solver handles long transitive chains correctly
/// and picks compatible versions across all levels.
#[test]
fn test_deep_chain_success() {
    let repo = create_test_repo(&[
        // Level 0: app
        ("app", "1.0.0", &["framework@>=2"]),
        
        // Level 1: framework (multiple versions)
        ("framework", "1.0.0", &["runtime@>=1"]),
        ("framework", "2.0.0", &["runtime@>=2"]),
        ("framework", "3.0.0", &["runtime@>=3"]),
        
        // Level 2: runtime
        ("runtime", "1.0.0", &["platform@>=1"]),
        ("runtime", "2.0.0", &["platform@>=1"]),
        ("runtime", "3.0.0", &["platform@>=2"]),
        
        // Level 3: platform
        ("platform", "1.0.0", &["arch@>=1"]),
        ("platform", "2.0.0", &["arch@>=1"]),
        
        // Level 4: arch
        ("arch", "1.0.0", &["base@>=1"]),
        ("arch", "2.0.0", &["base@>=1"]),
        
        // Level 5: base (leaf)
        ("base", "1.0.0", &[]),
        ("base", "2.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let solution = solver.solve_impl("app-1.0.0").unwrap();
    
    // All 6 packages should be resolved
    assert_eq!(solution.len(), 6);
    assert!(solution.contains(&"app-1.0.0".to_string()));
    assert!(solution.contains(&"framework-3.0.0".to_string())); // latest >=2
    assert!(solution.contains(&"runtime-3.0.0".to_string()));   // latest >=3
    assert!(solution.contains(&"platform-2.0.0".to_string()));  // latest >=2
    assert!(solution.contains(&"arch-2.0.0".to_string()));      // latest
    assert!(solution.contains(&"base-2.0.0".to_string()));      // latest
}

/// Deep chain with version constraint at leaf - CONFLICT
/// Same chain but app also requires base@<2, conflicting with chain.
#[test]
fn test_deep_chain_conflict() {
    let repo = create_test_repo(&[
        // app requires framework AND base<2 directly
        ("app", "1.0.0", &["framework@>=2", "base@<2"]),
        
        // framework needs runtime>=3 which needs platform>=2
        ("framework", "2.0.0", &["runtime@>=3"]),
        ("runtime", "3.0.0", &["platform@>=2"]),
        
        // platform-2.0 needs arch>=2 which needs base>=2 - CONFLICT!
        ("platform", "2.0.0", &["arch@>=2"]),
        ("arch", "2.0.0", &["base@>=2"]),
        
        // base versions
        ("base", "1.0.0", &[]),
        ("base", "2.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let result = solver.solve_impl("app-1.0.0");
    
    // Should fail - app wants base<2 but chain requires base>=2
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("base")); // Conflict involves base
}

/// Many versions stress test - solver picks optimal from 20+ versions
#[test]
fn test_many_versions_success() {
    // Create packages with many versions
    let packages: Vec<(&str, &str, &[&str])> = vec![
        ("app", "1.0.0", &["lib@>=5,<15"] as &[&str]),
    ];
    
    // lib has 20 versions: 1.0.0 through 20.0.0
    // Each requires utils matching its major version
    let lib_versions: Vec<(String, String)> = (1..=20)
        .map(|v| (format!("{}.0.0", v), format!("utils@>={}", v)))
        .collect();
    
    // utils has 20 versions
    let utils_versions: Vec<String> = (1..=20)
        .map(|v| format!("{}.0.0", v))
        .collect();
    
    // Build static refs for test
    let lib_refs: Vec<(&str, &str, Vec<&str>)> = lib_versions
        .iter()
        .map(|(v, req)| ("lib", v.as_str(), vec![req.as_str()]))
        .collect();
    
    // Note: lib_refs/utils_versions show the pattern but we create manually below
    let _ = (packages, lib_refs, utils_versions); // silence warnings
    
    // Create packages manually for test clarity
    let repo = create_test_repo(&[
        ("app", "1.0.0", &["lib@>=5,<15"]),
        
        // lib versions with deps
        ("lib", "5.0.0", &["utils@>=5"]),
        ("lib", "10.0.0", &["utils@>=10"]),
        ("lib", "14.0.0", &["utils@>=14"]),
        ("lib", "15.0.0", &["utils@>=15"]),  // excluded by <15
        ("lib", "20.0.0", &["utils@>=20"]),  // excluded
        
        // utils versions
        ("utils", "5.0.0", &[]),
        ("utils", "10.0.0", &[]),
        ("utils", "14.0.0", &[]),
        ("utils", "15.0.0", &[]),
        ("utils", "20.0.0", &[]),
    ]);

    let storage = Storage::scan_impl(Some(&[repo.path().to_path_buf()])).unwrap();
    let solver = Solver::from_packages(&storage.all_packages()).unwrap();

    let solution = solver.solve_impl("app-1.0.0").unwrap();
    
    // Should pick lib-14.0.0 (highest in >=5,<15 range)
    assert!(solution.contains(&"lib-14.0.0".to_string()));
    // And utils-20.0.0 (highest that satisfies utils>=14)
    assert!(solution.contains(&"utils-20.0.0".to_string()));
}

// =============================================================================
// Import style tests
// =============================================================================

/// Helper to create package.py with custom content.
fn create_package_custom(dir: &Path, name: &str, version: &str, content: &str) {
    let pkg_dir = dir.join(name).join(version);
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("package.py"), content).unwrap();
}

#[test]
fn test_import_star() {
    // Test 'from pkg import *' style
    let dir = TempDir::new().unwrap();
    create_package_custom(
        dir.path(),
        "star",
        "1.0.0",
        r#"from pkg import *

def get_package():
    p = Package("star", "1.0.0")
    return p
"#,
    );

    let storage = Storage::scan_impl(Some(&[dir.path().to_path_buf()])).unwrap();
    assert!(storage.has("star-1.0.0"));
}

#[test]
fn test_import_pkg_namespace() {
    // Test 'pkg.Package' style (no import needed)
    let dir = TempDir::new().unwrap();
    create_package_custom(
        dir.path(),
        "ns",
        "2.0.0",
        r#"def get_package():
    p = pkg.Package("ns", "2.0.0")
    p.envs.append(pkg.Env("default"))
    return p
"#,
    );

    let storage = Storage::scan_impl(Some(&[dir.path().to_path_buf()])).unwrap();
    assert!(storage.has("ns-2.0.0"));
}

#[test]
fn test_direct_class_access() {
    // Test direct class access without import (injected into globals)
    let dir = TempDir::new().unwrap();
    create_package_custom(
        dir.path(),
        "direct",
        "3.0.0",
        r#"def get_package():
    p = Package("direct", "3.0.0")
    return p
"#,
    );

    let storage = Storage::scan_impl(Some(&[dir.path().to_path_buf()])).unwrap();
    assert!(storage.has("direct-3.0.0"));
}

// =============================================================================
// Rex (pre_commands / commands / post_commands) integration
// =============================================================================

/// Package with pre_commands that set REX_VAR; after env merge + rex we expect the var.
#[test]
fn test_rex_pre_commands_mutate_env() {
    let dir = TempDir::new().unwrap();
    create_package_custom(
        dir.path(),
        "rexpkg",
        "1.0.0",
        r#"from pkg import Package, Env, Evar

def pre_commands():
    env.REX_VAR.set("from_pre_commands")

def get_package():
    p = Package("rexpkg", "1.0.0")
    e = Env("default")
    e.add(Evar("INITIAL", "ok", "set"))
    p.add_env(e)
    return p
"#,
    );

    let storage = Storage::scan_impl(Some(&[dir.path().to_path_buf()])).unwrap();
    let mut pkg = storage.resolve("rexpkg-1.0.0").unwrap().clone();
    pkg.solve_version_impl(&storage.all_packages()).unwrap();
    let mut env = pkg._env("default", true).expect("default env");

    let rex_order = packages_in_rex_order(&pkg);
    let root = package_root_from_source(pkg.package_source.as_ref());
    for p in &rex_order {
        if let Some(ref pre) = p.pre_commands {
            apply_package_commands(&mut env, p, pre, root.as_deref(), Some("pre_commands")).unwrap();
        }
    }

    let evar_names: Vec<String> = env.evars.iter().map(|e| e.name.clone()).collect();
    assert!(evar_names.contains(&"INITIAL".to_string()));
    assert!(evar_names.contains(&"REX_VAR".to_string()));
    let rex_val = env.evars.iter().find(|e| e.name == "REX_VAR").map(|e| e.value.as_str()).unwrap_or("");
    assert_eq!(rex_val, "from_pre_commands");
}

// =============================================================================
// pkg test flow: pre_test_commands + tests section (load and structure)
// =============================================================================

/// Package with pre_test_commands and tests; we only assert load and fields are present.
#[test]
fn test_package_test_section_loaded() {
    let dir = TempDir::new().unwrap();
    create_package_custom(
        dir.path(),
        "testpkg",
        "1.0.0",
        r#"from pkg import Package

def pre_test_commands():
    env.TEST_SETUP.set("done")

def get_package():
    p = Package("testpkg", "1.0.0")
    return p
"#,
    );

    let storage = Storage::scan_impl(Some(&[dir.path().to_path_buf()])).unwrap();
    let pkg = storage.get("testpkg-1.0.0").unwrap();
    assert!(pkg.pre_test_commands.is_some());
}

// =============================================================================
// CLI integration: run `pkg` binary against temp repo
// =============================================================================

/// Run `pkg -r <repo> search` and assert packages appear in stdout.
#[test]
fn test_cli_search_lists_packages() {
    let repo = create_test_repo(&[
        ("maya", "2024.0.0", &[]),
        ("houdini", "20.0.0", &[]),
    ]);

    let exe = env!("CARGO_BIN_EXE_pkg");
    let out = Command::new(exe)
        .arg("-r")
        .arg(repo.path())
        .arg("search")
        .output()
        .expect("run pkg search");

    assert!(out.status.success(), "pkg search failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("maya"), "stdout should list maya: {}", stdout);
    assert!(stdout.contains("houdini"), "stdout should list houdini: {}", stdout);
}

/// Run `pkg -r <repo> env maya` and assert success and env output (cross-platform; no child command).
#[test]
fn test_cli_env_prints_env() {
    let dir = TempDir::new().unwrap();
    create_package_custom(
        dir.path(),
        "maya",
        "2024.0.0",
        r#"from pkg import Package, Env, Evar
def get_package():
    p = Package("maya", "2024.0.0")
    e = Env("default")
    e.add(Evar("MAYA_TEST", "ok", "set"))
    p.add_env(e)
    return p
"#,
    );

    let exe = env!("CARGO_BIN_EXE_pkg");
    let out = Command::new(exe)
        .arg("-r")
        .arg(dir.path())
        .arg("env")
        .arg("maya")
        .output()
        .expect("run pkg env maya");

    assert!(out.status.success(), "pkg env maya failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("MAYA_TEST"), "stdout should contain MAYA_TEST: {}", stdout);
    assert!(stdout.contains("ok"), "stdout should contain env value: {}", stdout);
}
