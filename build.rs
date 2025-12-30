fn main() {
    pyo3_build_config::use_pyo3_cfgs();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // On Windows, DLLs are found via PATH or same directory - no rpath needed
    if target_os == "windows" {
        return;
    }

    // For Unix (macOS/Linux): set rpath so binary can find libpython at runtime
    let config = pyo3_build_config::get();
    if let Some(lib_dir) = &config.lib_dir {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir);
    }
}
