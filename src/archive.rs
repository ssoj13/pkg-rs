//! Zip archive utilities.

use jwalk::WalkDir;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use zip::write::FileOptions;

/// Extract a zip archive into a directory.
pub fn extract_zip(src_zip: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(src_zip).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel_path) = entry.enclosed_name() else {
            continue;
        };
        let out_path = dest_dir.join(rel_path);

        if entry.name().ends_with('/') {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut outfile = fs::File::create(&out_path).map_err(|e| e.to_string())?;
        io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;

        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let perm = std::fs::Permissions::from_mode(mode);
            let _ = std::fs::set_permissions(&out_path, perm);
        }
    }

    Ok(())
}

/// Create a zip archive from a directory (recursively).
pub fn zip_dir(src_dir: &Path, dest_zip: &Path, compression_level: Option<i64>) -> Result<(), String> {
    if !src_dir.is_dir() {
        return Err(format!("zip_dir: source is not a directory: {}", src_dir.display()));
    }
    if let Some(parent) = dest_zip.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = fs::File::create(dest_zip).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);

    let mut entries: Vec<(PathBuf, bool)> = WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| (e.path(), e.file_type().is_dir()))
        .filter(|(path, _)| path != src_dir)
        .collect();

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (path, is_dir) in entries {
        let rel = path
            .strip_prefix(src_dir)
            .map_err(|e| e.to_string())?;
        if rel.as_os_str().is_empty() {
            continue;
        }

        let mut name = rel.to_string_lossy().replace('\\', "/");
        if is_dir && !name.ends_with('/') {
            name.push('/');
        }

        #[cfg(unix)]
        let options = {
            let mut opt = FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(compression_level);
            if let Ok(meta) = fs::metadata(&path) {
                use std::os::unix::fs::PermissionsExt;
                opt = opt.unix_permissions(meta.permissions().mode());
            }
            opt
        };
        #[cfg(not(unix))]
        let options = FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(compression_level);

        if is_dir {
            zip.add_directory(name, options).map_err(|e| e.to_string())?;
            continue;
        }

        zip.start_file(name, options).map_err(|e| e.to_string())?;
        let mut infile = fs::File::open(&path).map_err(|e| e.to_string())?;
        io::copy(&mut infile, &mut zip).map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
