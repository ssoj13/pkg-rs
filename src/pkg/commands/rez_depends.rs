//! rez depends command (native).

use crate::cli::DependsArgs;
use pkg_lib::{DepSpec, Package, Storage};
use std::collections::HashSet;
use std::process::ExitCode;

/// Show dependency graph in DOT, Mermaid, or list format.
/// When packages is empty, show graph for entire repository.
pub fn cmd_rez_depends(storage: &Storage, args: &DependsArgs) -> ExitCode {
    let mut edges: Vec<(String, String)> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut roots: Vec<String> = Vec::new();

    if args.packages.is_empty() {
        for pkg in storage.packages() {
            if args.reverse {
                collect_reverse_deps(storage, &pkg.base, &mut edges, &mut visited, 0, args.depth);
            } else {
                collect_deps(storage, &pkg, &mut edges, &mut visited, 0, args.depth);
            }
        }
    } else {
        for name in &args.packages {
            let Some(pkg) = storage.resolve(name) else {
                eprintln!("Package not found: {}", name);
                return ExitCode::FAILURE;
            };
            roots.push(pkg.name.clone());

            if args.reverse {
                collect_reverse_deps(storage, &pkg.base, &mut edges, &mut visited, 0, args.depth);
            } else {
                collect_deps(storage, &pkg, &mut edges, &mut visited, 0, args.depth);
            }
        }
    }

    match args.format.as_str() {
        "list" => print_list(&roots, &edges),
        "dot" => print_dot(&roots, &edges),
        "mermaid" => print_mermaid(&roots, &edges),
        other => {
            eprintln!("Unknown format: {}. Use 'list', 'dot', or 'mermaid'", other);
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

/// Collect forward dependencies recursively.
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

/// Collect reverse dependencies (what depends on this package).
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

/// Print list format: one line per root with its direct edges.
fn print_list(roots: &[String], edges: &[(String, String)]) {
    for root in roots {
        let deps: Vec<String> = edges
            .iter()
            .filter_map(|(a, b)| if a == root { Some(b.clone()) } else { None })
            .collect();
        if deps.is_empty() {
            println!("{}", root);
        } else {
            println!("{}: {}", root, deps.join(", "));
        }
    }
}

/// Print graph in DOT format (Graphviz).
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

/// Print graph in Mermaid format.
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
