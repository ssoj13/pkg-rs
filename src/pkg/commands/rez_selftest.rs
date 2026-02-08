//! Rez selftest command.

use crate::cli::SelftestArgs;
use pkg_lib::{config, Solver, Storage};
use std::process::ExitCode;

pub fn cmd_rez_selftest(storage: &Storage, args: &SelftestArgs) -> ExitCode {
    let mut ok = true;

    if config::get().is_err() {
        eprintln!("selftest: failed to load config");
        ok = false;
    } else if args.verbose {
        if let Ok(cfg) = config::get() {
            eprintln!("selftest: config loaded ({} files)", cfg.sourced_filepaths.len());
        }
    }

    if args.verbose {
        eprintln!("selftest: storage packages={} repos={}", storage.count(), storage.locations().len());
    }

    if storage.count() > 0 {
        let packages = storage.packages();
        if let Ok(solver) = Solver::new(packages.clone()) {
            // try a trivial solve on the first package
            if let Some(pkg) = packages.first() {
                if solver.solve(&pkg.name).is_err() {
                    eprintln!("selftest: solver failed on {}", pkg.name);
                    ok = false;
                }
            }
        } else {
            eprintln!("selftest: solver init failed");
            ok = false;
        }
    }

    if ok {
        println!("selftest ok");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
