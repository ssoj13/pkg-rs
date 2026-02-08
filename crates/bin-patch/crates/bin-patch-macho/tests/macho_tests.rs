//! Tests for remap_bundle_paths (in-place install_name and rpath remap).

use bin_patch_macho::MachoContainer;
use goblin::mach::MachO;
use std::path::PathBuf;

/// Remap rpath and re-parse: new string must fit in existing slot.
#[test]
fn remap_bundle_paths_rpath() {
    let bin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/data/macho/x64/exec/hello_with_rpath");
    if !bin_path.exists() {
        eprintln!("skip (no test data): {}", bin_path.display());
        return;
    }
    let data = std::fs::read(&bin_path).unwrap();
    let mut container = MachoContainer::parse(&data).unwrap();
    let rpath_remaps = vec![("path_graf".to_string(), "@x".to_string())];
    container.remap_bundle_paths(&[], &rpath_remaps).unwrap();
    let parsed = MachO::parse(&container.data, 0).unwrap();
    assert!(
        parsed.rpaths.iter().any(|r| *r == "@x"),
        "rpaths after remap: {:?}",
        parsed.rpaths
    );
}

/// Remap install_name to shorter @loader_path style (bundle reloc).
#[test]
fn remap_bundle_paths_install_name() {
    let bin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/data/macho/x64/exec/hello_with_rpath");
    if !bin_path.exists() {
        eprintln!("skip (no test data): {}", bin_path.display());
        return;
    }
    let data = std::fs::read(&bin_path).unwrap();
    let mut container = MachoContainer::parse(&data).unwrap();
    // Slot for /usr/lib/libSystem.B.dylib is long; replace with shorter path.
    let install_remaps = vec![(
        "/usr/lib/libSystem.B.dylib".to_string(),
        "@loader_path/libSystem.B".to_string(),
    )];
    container.remap_bundle_paths(&install_remaps, &[]).unwrap();
    let parsed = MachO::parse(&container.data, 0).unwrap();
    assert!(
        parsed.libs.iter().any(|l| *l == "@loader_path/libSystem.B"),
        "libs after remap: {:?}",
        parsed.libs
    );
}
