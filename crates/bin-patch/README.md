# bin-patch

ELF and Mach-O in-place path patching for relocatable bundles (used by pkg-rs). No patchelf, no install_name_tool.

| Crate              | Platform | Role |
|--------------------|----------|------|
| **bin-patch-elf**  | Linux    | ELF runpath remap |
| **bin-patch-macho**| macOS    | Mach-O: `parse()` + `remap_bundle_paths(install_name_remaps, rpath_remaps)` (one parse, in-place). Single/fat arch. |

Both are built as path deps from pkg-rs root.
