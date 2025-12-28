//! Benchmarks for storage scanning and solving.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use pkg_lib::{Solver, Storage};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create a test package in the given directory.
fn create_package(dir: &Path, name: &str, version: &str, requires: &[&str]) {
    let pkg_dir = dir.join(name).join(version);
    fs::create_dir_all(&pkg_dir).unwrap();

    let reqs = if requires.is_empty() {
        String::new()
    } else {
        let reqs_str: Vec<String> = requires.iter().map(|r| format!("\"{}\"", r)).collect();
        format!("\n    pkg.add_req({})", reqs_str.join(")\n    pkg.add_req("))
    };

    let content = format!(
        r#"from pkg import Package

def get_package():
    pkg = Package("{}", "{}"){} 
    return pkg
"#,
        name, version, reqs
    );

    fs::write(pkg_dir.join("package.py"), content).unwrap();
}

/// Create a test repo with N packages.
fn create_test_repo(n: usize) -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create base package
    create_package(dir.path(), "base", "1.0.0", &[]);

    // Create N packages that depend on base
    for i in 0..n {
        create_package(dir.path(), &format!("pkg{}", i), "1.0.0", &["base@1"]);
    }

    dir
}

/// Create a dependency chain: pkg0 -> pkg1 -> pkg2 -> ... (depth levels).
fn create_chain_repo(depth: usize) -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create leaf package (no deps)
    create_package(dir.path(), &format!("pkg{}", depth - 1), "1.0.0", &[]);

    // Create chain: each package depends on the next
    for i in (0..depth - 1).rev() {
        let dep = format!("pkg{}@1", i + 1);
        create_package(dir.path(), &format!("pkg{}", i), "1.0.0", &[&dep]);
    }

    dir
}

fn bench_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_scan");

    for size in [10, 50, 100, 200] {
        let dir = create_test_repo(size);
        let paths = vec![dir.path().to_path_buf()];

        // Clear cache for cold scan
        if let Some(cache_path) = pkg_lib::cache::Cache::cache_path() {
            let _ = fs::remove_file(&cache_path);
        }

        group.bench_with_input(BenchmarkId::new("cold", size), &size, |b, _| {
            b.iter(|| {
                // Clear cache each iteration
                if let Some(cache_path) = pkg_lib::cache::Cache::cache_path() {
                    let _ = fs::remove_file(&cache_path);
                }
                let storage = Storage::scan_impl(Some(&paths)).unwrap();
                black_box(storage)
            });
        });

        // Warm scan (cache populated)
        let _ = Storage::scan_impl(Some(&paths)).unwrap();

        group.bench_with_input(BenchmarkId::new("warm", size), &size, |b, _| {
            b.iter(|| {
                let storage = Storage::scan_impl(Some(&paths)).unwrap();
                black_box(storage)
            });
        });
    }

    group.finish();
}

fn bench_solve(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver");

    // Benchmark solving with varying number of requirements
    for n in [5, 10, 20, 50] {
        let dir = create_test_repo(n);
        let paths = vec![dir.path().to_path_buf()];
        let storage = Storage::scan_impl(Some(&paths)).unwrap();
        let solver = Solver::from_packages(&storage.all_packages()).unwrap();

        // Create requirements for half the packages
        let reqs: Vec<String> = (0..n / 2).map(|i| format!("pkg{}", i)).collect();

        group.bench_with_input(BenchmarkId::new("requirements", n / 2), &n, |b, _| {
            b.iter(|| {
                let result = solver.solve_requirements_impl(&reqs);
                black_box(result)
            });
        });
    }

    // Benchmark chain depth
    for depth in [5, 10, 20] {
        let dir = create_chain_repo(depth);
        let paths = vec![dir.path().to_path_buf()];
        let storage = Storage::scan_impl(Some(&paths)).unwrap();
        let solver = Solver::from_packages(&storage.all_packages()).unwrap();

        group.bench_with_input(BenchmarkId::new("chain_depth", depth), &depth, |b, _| {
            b.iter(|| {
                let result = solver.solve_impl("pkg0-1.0.0");
                black_box(result)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_scan, bench_solve);
criterion_main!(benches);
