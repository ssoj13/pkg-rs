//! rez memcache command (native, config introspection).

use crate::cli::MemcacheArgs;
use pkg_lib::config;
use std::process::ExitCode;

pub fn cmd_rez_memcache(args: &MemcacheArgs) -> ExitCode {
    let cfg = match config::get() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("rez memcache: {}", err);
            return ExitCode::FAILURE;
        }
    };

    let uris = config::get_json(cfg, "memcached_uri")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "[]".to_string());

    if args.clear {
        println!("Memcache clear requested (memcache is not active in pkg-rs yet).");
        println!("Configured memcached_uri: {}", uris);
        return ExitCode::SUCCESS;
    }

    if args.stats {
        println!("Memcache status:");
        println!("  configured memcached_uri: {}", uris);
        println!("  active: false");
        return ExitCode::SUCCESS;
    }

    println!("Memcache not enabled. configured memcached_uri: {}", uris);
    ExitCode::SUCCESS
}
