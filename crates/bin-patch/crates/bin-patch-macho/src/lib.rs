//! Mach-O in-place path remap for relocatable bundles.
//!
//! Single API: parse bytes, then `remap_bundle_paths(install_name_remaps, rpath_remaps)`.
//! New strings must not exceed existing load command slot length (e.g. `@loader_path/...` for bundles).

pub mod container;
pub mod error;
pub mod patcher;

pub use container::*;
pub use error::MachoError;
