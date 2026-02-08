//! Helpers for in-place Mach-O path remap (no load command insert/remove).

use goblin::mach::load_command::{self, LoadCommand, RpathCommand};
use goblin::mach::load_command::{CommandVariant::*, DylibCommand};

use crate::error::MachoError;

pub fn find_rpath_command(
    commands: &[load_command::LoadCommand],
    index: usize,
) -> Option<(&LoadCommand, &RpathCommand)> {
    let mut count = 0;
    for command in commands {
        if let Rpath(rpath_command) = &command.command {
            if count == index {
                return Some((command, rpath_command));
            }
            count += 1;
        }
    }
    None
}

pub fn find_dylib_command(
    commands: &[load_command::LoadCommand],
    index: usize,
) -> Option<(&LoadCommand, &DylibCommand)> {
    let mut count = 0;
    for command in commands {
        match &command.command {
            LoadDylib(d)
            | LoadUpwardDylib(d)
            | ReexportDylib(d)
            | LoadWeakDylib(d)
            | LazyLoadDylib(d) => {
                if count == index {
                    return Some((command, d));
                }
                count += 1;
            }
            _ => {}
        }
    }
    None
}

/// In-place replace a null-terminated string. Fills remainder with zeros. Fails if new_str.len() + 1 > max_len.
pub fn replace_cstring_in_place(
    buffer: &mut [u8],
    offset: usize,
    new_str: &str,
    max_len: usize,
) -> Result<(), MachoError> {
    let need = new_str.len() + 1;
    if need > max_len {
        return Err(MachoError::RemapStringTooLong(need, max_len));
    }
    let slot = buffer
        .get_mut(offset..)
        .and_then(|s| s.get_mut(..max_len))
        .ok_or(MachoError::RemapBufferTooShort)?;
    slot[..new_str.len()].copy_from_slice(new_str.as_bytes());
    slot[new_str.len()] = 0;
    if need < max_len {
        slot[need..].fill(0);
    }
    Ok(())
}
