//! Mach-O parse and in-place path remap for relocatable bundles (e.g. pkg-rs).

use goblin::{
    container,
    mach::{
        fat::{self, FatArch},
        parse_magic_and_ctx, peek, MachO, MultiArch, SingleArch,
    },
};

use crate::{
    error::MachoError,
    patcher::{find_dylib_command, find_rpath_command, replace_cstring_in_place},
};

pub struct SingleMachO<'a> {
    pub inner: MachO<'a>,
    pub data: Vec<u8>,
    pub ctx: container::Ctx,
}

impl SingleMachO<'_> {
    /// In-place remap of install_name and rpath strings. New string must not exceed existing slot length.
    pub fn apply_remaps_in_place(
        &mut self,
        install_name_remaps: &[(String, String)],
        rpath_remaps: &[(String, String)],
    ) -> Result<(), MachoError> {
        let name_map: std::collections::HashMap<_, _> =
            install_name_remaps.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let rpath_map: std::collections::HashMap<_, _> =
            rpath_remaps.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        const DYLIB_NAME_OFFSET: usize = 24;
        const RPATH_PATH_OFFSET: usize = 12;

        for (dylib_idx, lib_name) in self.inner.libs.iter().enumerate() {
            let Some((load_command, _)) = find_dylib_command(&self.inner.load_commands, dylib_idx)
            else {
                continue;
            };
            let Some(&new_name) = name_map.get(&**lib_name) else {
                continue;
            };
            let name_offset = load_command.offset + DYLIB_NAME_OFFSET;
            let slot_len = load_command.command.cmdsize() as usize - DYLIB_NAME_OFFSET;
            replace_cstring_in_place(&mut self.data, name_offset, new_name, slot_len)?;
        }

        for (rpath_idx, old_rpath) in self.inner.rpaths.iter().enumerate() {
            let Some((load_command, _)) = find_rpath_command(&self.inner.load_commands, rpath_idx)
            else {
                continue;
            };
            let Some(&new_rpath) = rpath_map.get(&**old_rpath) else {
                continue;
            };
            let path_offset = load_command.offset + RPATH_PATH_OFFSET;
            let slot_len = load_command.command.cmdsize() as usize - RPATH_PATH_OFFSET;
            replace_cstring_in_place(&mut self.data, path_offset, new_rpath, slot_len)?;
        }

        Ok(())
    }
}

pub struct FatMacho<'a> {
    pub inner: SingleMachO<'a>,
    pub arch: FatArch,
}

pub struct FatMachoContainer<'a> {
    pub archs: Vec<FatMacho<'a>>,
    pub data: Vec<u8>,
}

#[allow(clippy::large_enum_variant)]
pub enum MachoType<'a> {
    SingleArch(SingleMachO<'a>),
    Fat(FatMachoContainer<'a>),
}

pub struct MachoContainer<'a> {
    pub inner: MachoType<'a>,
    pub data: Vec<u8>,
}

impl MachoContainer<'_> {
    /// One-parse batch remap: all install_name and rpath replacements in place (per arch). New strings must fit in existing slots.
    pub fn remap_bundle_paths(
        &mut self,
        install_name_remaps: &[(String, String)],
        rpath_remaps: &[(String, String)],
    ) -> Result<(), MachoError> {
        if install_name_remaps.is_empty() && rpath_remaps.is_empty() {
            return Ok(());
        }
        match &mut self.inner {
            MachoType::SingleArch(single) => {
                single.apply_remaps_in_place(install_name_remaps, rpath_remaps)?;
                self.data = single.data.clone();
            }
            MachoType::Fat(fat) => {
                for macho in &mut fat.archs {
                    macho.inner.apply_remaps_in_place(install_name_remaps, rpath_remaps)?;
                    let arch = macho.arch;
                    self.data.splice(
                        arch.offset as usize..arch.offset as usize + arch.size as usize,
                        macho.inner.data.clone(),
                    );
                }
            }
        }
        Ok(())
    }
}

impl<'a> MachoContainer<'a> {
    pub fn parse(bytes_of_file: &'a [u8]) -> Result<Self, MachoError> {
        let magic = peek(bytes_of_file, 0)?;
        match magic {
            fat::FAT_MAGIC => {
                let multi_arch = MultiArch::new(bytes_of_file)?;
                let mut archs = Vec::new();
                for arch in multi_arch.iter_arches() {
                    archs.push(arch?);
                }
                let mut machos = Vec::new();
                for (idx, arch) in multi_arch.into_iter().enumerate() {
                    let single = arch?;
                    let SingleArch::MachO(mach_o) = single else {
                        return Err(MachoError::UnixArchive);
                    };
                    let fat_arch = archs.get(idx).unwrap();
                    let data = fat_arch.slice(bytes_of_file);
                    let (_, maybe_ctx) = parse_magic_and_ctx(data, 0)?;
                    let ctx = maybe_ctx.ok_or(MachoError::UnknownEndian)?;
                    machos.push(FatMacho {
                        inner: SingleMachO {
                            inner: mach_o,
                            data: data.to_vec(),
                            ctx,
                        },
                        arch: *fat_arch,
                    });
                }
                Ok(MachoContainer {
                    inner: MachoType::Fat(FatMachoContainer {
                        archs: machos,
                        data: bytes_of_file.to_vec(),
                    }),
                    data: bytes_of_file.to_vec(),
                })
            }
            _ => {
                let mach_o = MachO::parse(bytes_of_file, 0)?;
                let (_, maybe_ctx) = parse_magic_and_ctx(bytes_of_file, 0)?;
                let ctx = maybe_ctx.ok_or(MachoError::UnknownEndian)?;
                Ok(MachoContainer {
                    inner: MachoType::SingleArch(SingleMachO {
                        inner: mach_o,
                        data: bytes_of_file.to_vec(),
                        ctx,
                    }),
                    data: bytes_of_file.to_vec(),
                })
            }
        }
    }
}
