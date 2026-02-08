//! Legacy pkg-rs command implementations (hidden under `pkg legacy`).

mod list;
mod info;
mod graph;
mod scan;
mod generate;
mod gen_pkg;

pub use list::{cmd_list, matches_glob};
pub use info::cmd_info;
pub use graph::cmd_graph;
pub use scan::cmd_scan;
pub use generate::cmd_generate_repo;
pub use gen_pkg::cmd_gen_pkg;
