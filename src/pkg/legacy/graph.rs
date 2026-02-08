//! Graph visualization command.

use pkg_lib::{Package, Storage};
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
        // No packages specified - show entire repo graph
        for pkg in storage.packages() {
            collect_deps(storage, &pkg, &mut edges, &mut visited, 0, max_depth);
        }
    } else {
        // Specific packages
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

    // Output in requested format
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
        let dep_base = if req.contains('@') {
            req.split('@').next().unwrap_or(req)
        } else {
            req.as_str()
        };

        edges.push((pkg.name.clone(), dep_base.to_string()));

        if let Some(dep_pkg) = storage.resolve(dep_base) {
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
            let dep_base = if req.contains('@') {
                req.split('@').next().unwrap_or(req)
            } else {
                req.as_str()
            };

            if dep_base == base {
                edges.push((pkg.name.clone(), base.to_string()));
                collect_reverse_deps(storage, &pkg.base, edges, visited, depth + 1, max_depth);
            }
        }
    }
}

/// Print graph in DOT format (Graphviz).
fn print_dot(roots: &[String], edges: &[(String, String)]) {
    println!("digraph deps {{");
    println!("  rankdir=LR;");
    println!("  node [shape=box, style=filled, fillcolor=lightblue];");
    
    for root in roots {
        println!("  \"{}\" [fillcolor=orange];", root);
    }
    
    for (from, to) in edges {
        println!("  \"{}\" -> \"{}\";", from, to);
    }
    println!("}}");
}

/// Print graph in Mermaid format.
fn print_mermaid(roots: &[String], edges: &[(String, String)]) {
    println!("```mermaid");
    println!("graph LR");
    
    for root in roots {
        println!("  {}[{}]:::root", sanitize_mermaid(root), root);
    }
    
    for (from, to) in edges {
        let from_id = sanitize_mermaid(from);
        let to_id = sanitize_mermaid(to);
        println!("  {}[{}] --> {}[{}]", from_id, from, to_id, to);
    }
    
    println!("  classDef root fill:#f96,stroke:#333");
    println!("```");
}

/// Sanitize node ID for Mermaid.
fn sanitize_mermaid(s: &str) -> String {
    s.replace('-', "_").replace('.', "_").replace('@', "_")
}
