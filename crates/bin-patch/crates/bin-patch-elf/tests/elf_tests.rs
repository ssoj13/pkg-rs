//! ELF runpath tests. Skip when test data is missing (tests/data/elf/...).

use bin_patch_elf::ElfContainer;
use goblin::elf::Elf;
use std::path::PathBuf;

#[test]
fn elf_runpath_set_roundtrip() {
    let bin_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/data/elf/x64/exec/linux-x64-bash");
    if !bin_path.exists() {
        eprintln!("skip (no test data): {}", bin_path.display());
        return;
    }
    let data = std::fs::read(&bin_path).unwrap();
    let mut container = ElfContainer::parse(&data).unwrap();
    container
        .set_runpath("path_graf_path_graf_path_graf_path_graf")
        .unwrap();
    let mut out = Vec::new();
    container.write(&mut out).unwrap();
    let parsed = Elf::parse(&out).unwrap();
    assert!(parsed.runpaths.iter().any(|r| *r == "path_graf_path_graf_path_graf_path_graf"));
}
