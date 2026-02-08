//! Dependency graph (legacy implementation).

use pkg_lib::{DepSpec, Package, Storage};
use std::collections::HashSet;
use std::process::ExitCode;

/// Show dependency graph in DOT or Mermaid format.
pub fn cmd_graph(
    storage: &Storage,
    packages: Vec<String>,
    format: &str,
    max_depth: usize,
    reverse: bool,
) -> ExitCode {
    let mut edges: Vec<(String, String)> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut roots: Vec<String> = Vec::new();

    if packages.is_empty() {
        for pkg in storage.packages() {
            if reverse {
                collect_reverse_deps(storage, &pkg.base, &mut edges, &mut visited, 0, max_depth);
            } else {
                collect_deps(storage, &pkg, &mut edges, &mut visited, 0, max_depth);
            }
        }
    } else {
        for name in &packages {
            let Some(pkg) = storage.resolve(name) else {
                eprintln!("Package not found: {}", name);
                return ExitCode::FAILURE;
            };
            roots.push(pkg.name.clone());

            if reverse {
                collect_reverse_deps(storage, &pkg.base, &mut edges, &mut visited, 0, max_depth);
            } else {
                collect_deps(storage, &pkg, &mut edges, &mut visited, 0, max_depth);
            }
        }
    }

    match format {
        "dot" => print_dot(&roots, &edges),
        "mermaid" => print_mermaid(&roots, &edges),
        _ => {
            eprintln!("Unknown format: {}. Use 'dot' or 'mermaid'", format);
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn collect_deps(
    storage: &Storage,
    pkg: &Package,
    edges: &mut Vec<(String, String)>,
    visited: &mut HashSet<String>,
    depth: usize,
    max_depth: usize,
) {
    if max_depth > 0 && depth >= max_depth {
        return;
    }
    if !visited.insert(pkg.name.clone()) {
        return;
    }

    for req in &pkg.reqs {
        let dep_base = match DepSpec::parse_impl(req) {
            Ok(spec) => spec.base,
            Err(_) => continue,
        };

        edges.push((pkg.name.clone(), dep_base.clone()));

        if let Some(dep_pkg) = storage.resolve(&dep_base) {
            collect_deps(storage, &dep_pkg, edges, visited, depth + 1, max_depth);
        }
    }
}

fn collect_reverse_deps(
    storage: &Storage,
    base: &str,
    edges: &mut Vec<(String, String)>,
    visited: &mut HashSet<String>,
    depth: usize,
    max_depth: usize,
) {
    if max_depth > 0 && depth >= max_depth {
        return;
    }
    if !visited.insert(base.to_string()) {
        return;
    }

    for pkg in storage.packages() {
        for req in &pkg.reqs {
            let dep_base = match DepSpec::parse_impl(req) {
                Ok(spec) => spec.base,
                Err(_) => continue,
            };

            if dep_base == base {
                edges.push((pkg.name.clone(), base.to_string()));
                collect_reverse_deps(storage, &pkg.base, edges, visited, depth + 1, max_depth);
            }
        }
    }
}

fn print_dot(roots: &[String], edges: &[(String, String)]) {
    println!("digraph deps {{");
    println!("  rankdir=LR;");
    for root in roots {
        println!("  \"{}\" [shape=box];", root);
    }
    for (a, b) in edges {
        println!("  \"{}\" -> \"{}\";", a, b);
    }
    println!("}}");
}

fn print_mermaid(roots: &[String], edges: &[(String, String)]) {
    println!("graph LR");
    for root in roots {
        println!("  {}[{}]", sanitize(root), root);
    }
    for (a, b) in edges {
        println!("  {} --> {}", sanitize(a), sanitize(b));
    }
}

fn sanitize(name: &str) -> String {
    name.replace('-', "_").replace('.', "_")
}
