//! Legacy pkg-rs commands. Not used by the main pkg binary; enable via feature if needed.

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
