//! Bundle library patching: remap ELF rpath (Linux) and Mach-O install_name/rpath (macOS)
//! so that bundled binaries find libs inside the bundle.
//! Pure Rust via bin-patch crates (no patchelf, no install_name_tool).

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs;
use std::path::{Path, PathBuf};

/// Patch binaries in the bundle so rpath/install_name point into the bundle.
/// `pairs`: (src_root, dest_root) for each relocated package.
pub fn patch_bundle_libs(_bundle_root: &Path, pairs: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    if pairs.is_empty() {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    return patch_elf(pairs);
    #[cfg(target_os = "macos")]
    return patch_macho(pairs);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pairs;
        Ok(())
    }
}

/// Path A is under prefix B (lexical).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn path_under(path: &Path, prefix: &Path) -> bool {
    path.strip_prefix(prefix).is_ok()
}

/// Relative path from `from` to `to` (e.g. from dir of binary to lib).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn path_relative_from(from: &Path, to: &Path) -> PathBuf {
    let from_c: Vec<_> = from.components().collect();
    let to_c: Vec<_> = to.components().collect();
    let common = from_c
        .iter()
        .zip(to_c.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let up = from_c.len().saturating_sub(common);
    let down = to_c.get(common..).unwrap_or(&[]);
    let mut p = PathBuf::new();
    for _ in 0..up {
        p.push("..");
    }
    for c in down {
        p.push(c);
    }
    p
}

#[cfg(target_os = "linux")]
fn patch_elf(pairs: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    use bin_patch_elf::rewriter::Writer;
    use std::os::unix::fs::PermissionsExt;

    let mut patched = 0u32;
    for (_src, dest_root) in pairs {
        for path in find_elf_files(dest_root) {
            let data = match fs::read(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let mut writer = match Writer::read(&data) {
                Ok(w) => w,
                Err(_) => continue,
            };
            let runpath_bytes = match writer.elf_runpath() {
                Some(r) => r,
                None => continue,
            };
            let rpath_str = String::from_utf8_lossy(runpath_bytes);
            let rpaths: Vec<String> = rpath_str.split(':').map(String::from).collect();
            if rpaths.is_empty() {
                continue;
            }
            let elf_dir = path.parent().unwrap_or(Path::new("."));
            let mut new_rpaths = Vec::with_capacity(rpaths.len());
            let mut changed = false;
            for rpath in &rpaths {
                let rpath_path = Path::new(rpath);
                if !rpath_path.is_absolute() {
                    new_rpaths.push(rpath.clone());
                    continue;
                }
                let mut new_rpath = None;
                for (src_root, dest_root) in pairs {
                    if path_under(rpath_path, src_root) {
                        let rel = path_relative_from(rpath_path, src_root);
                        let new_abs = dest_root.join(rel);
                        let new_rel = path_relative_from(elf_dir, &new_abs);
                        new_rpath = Some(format!("$ORIGIN/{}", new_rel.display()));
                        break;
                    }
                }
                if let Some(n) = new_rpath {
                    new_rpaths.push(n);
                    changed = true;
                } else {
                    new_rpaths.push(rpath.clone());
                }
            }
            if !changed {
                continue;
            }
            let new_runpath = new_rpaths.join(":");
            writer
                .elf_set_runpath(new_runpath.into_bytes())
                .map_err(|e| e.to_string())?;
            writer
                .write_to_path(&path)
                .map_err(|e| e.to_string())?;
            patched += 1;
        }
    }
    if patched > 0 {
        log::info!("bundle_patch: patched {} ELF(s)", patched);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn find_elf_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else { return out };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            out.extend(find_elf_files(&path));
            continue;
        }
        if path.is_symlink() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_so = name.contains(".so") || name.ends_with(".so");
        let meta = match e.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let executable = meta.permissions().mode() & 0o111 != 0;
        if !is_so && !executable {
            continue;
        }
        let Ok(buf) = fs::read(&path) else { continue };
        if buf.len() >= 4 && &buf[0..4] == b"\x7fELF" {
            out.push(path);
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn patch_macho(pairs: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    use bin_patch_macho::MachoContainer;
    use goblin::mach::{Mach, SingleArch};
    use goblin::Object;

    let mut patched = 0u32;
    for (_src, dest_root) in pairs {
        for path in find_macho_files(dest_root) {
            let data = match fs::read(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let obj = match Object::parse(&data) {
                Ok(o) => o,
                Err(_) => continue,
            };
            let (libs, rpaths) = match obj {
                Object::Mach(mach) => {
                    let mut libs = Vec::new();
                    let mut rpaths = Vec::new();
                    match mach {
                        Mach::Binary(m) => {
                            libs.extend(m.libs.iter().map(|s| s.to_string()));
                            rpaths.extend(m.rpaths.iter().map(|s| s.to_string()));
                        }
                        Mach::Fat(multi) => {
                            for entry in multi.into_iter().flatten() {
                                if let SingleArch::MachO(m) = entry.1 {
                                    libs.extend(m.libs.iter().map(|s| s.to_string()));
                                    rpaths.extend(m.rpaths.iter().map(|s| s.to_string()));
                                }
                            }
                        }
                    }
                    (libs, rpaths)
                }
                _ => continue,
            };
            let elf_dir = path.parent().unwrap_or(Path::new("."));

            let mut lib_changes: Vec<(String, String)> = Vec::new();
            for lib in &libs {
                let lib_path = Path::new(lib);
                if !lib_path.is_absolute() || lib.starts_with('@') {
                    continue;
                }
                for (src_root, dest_root) in pairs {
                    if path_under(lib_path, src_root) {
                        let rel = path_relative_from(lib_path, src_root);
                        let new_abs = dest_root.join(rel);
                        let new_rel = path_relative_from(elf_dir, &new_abs);
                        let new_name = format!("@loader_path/{}", new_rel.display());
                        lib_changes.push((lib.clone(), new_name));
                        break;
                    }
                }
            }
            let mut rpath_changes: Vec<(String, String)> = Vec::new();
            for rpath in &rpaths {
                let rpath_path = Path::new(rpath);
                if !rpath_path.is_absolute() {
                    continue;
                }
                for (src_root, dest_root) in pairs {
                    if path_under(rpath_path, src_root) {
                        let rel = path_relative_from(rpath_path, src_root);
                        let new_abs = dest_root.join(rel);
                        let new_rel = path_relative_from(elf_dir, &new_abs);
                        rpath_changes.push((rpath.clone(), format!("@loader_path/{}", new_rel.display())));
                        break;
                    }
                }
            }
            if lib_changes.is_empty() && rpath_changes.is_empty() {
                continue;
            }

            let mut container = MachoContainer::parse(&data).map_err(|e| e.to_string())?;
            container
                .remap_bundle_paths(&lib_changes, &rpath_changes)
                .map_err(|e| e.to_string())?;
            patched += 1;
            fs::write(&path, container.data).map_err(|e| e.to_string())?;
        }
    }
    if patched > 0 {
        log::info!("bundle_patch: patched {} Mach-O reference(s)", patched);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn find_macho_files(root: &Path) -> Vec<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else { return out };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            out.extend(find_macho_files(&path));
            continue;
        }
        if path.is_symlink() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_dylib = name.ends_with(".dylib") || name.contains(".so");
        let Ok(buf) = fs::read(&path) else { continue };
        if buf.len() < 4 {
            continue;
        }
        let magic = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let is_macho = magic == 0xFEEDFACE || magic == 0xFEEDFACF;
        let executable = e.metadata().map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false);
        if is_macho && (is_dylib || executable) {
            out.push(path);
        }
    }
    out
}
