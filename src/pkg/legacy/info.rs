//! Show package info command.

use pkg_lib::Storage;
use std::process::ExitCode;

/// Show detailed package information.
pub fn cmd_info(storage: &Storage, package: &str, json: bool) -> ExitCode {
    let pkg = storage.resolve(package);

    let Some(pkg) = pkg else {
        eprintln!("Package not found: {}", package);
        return ExitCode::FAILURE;
    };

    if json {
        println!("{}", pkg.to_json_pretty().unwrap_or_default());
    } else {
        println!("Package: {}", pkg.name);
        println!("  Base: {}", pkg.base);
        println!("  Version: {}", pkg.version);

        if !pkg.reqs.is_empty() {
            println!("  Requirements:");
            for req in &pkg.reqs {
                println!("    - {}", req);
            }
        }

        if !pkg.envs.is_empty() {
            println!("  Environments:");
            for env in &pkg.envs {
                println!("    - {} ({} vars)", env.name, env.evars.len());
            }
        }

        if !pkg.apps.is_empty() {
            println!("  Applications:");
            for app in &pkg.apps {
                let path_info = app.path.as_deref().unwrap_or("(no path)");
                println!("    - {}: {}", app.name, path_info);
            }
        }
    }

    ExitCode::SUCCESS
}
