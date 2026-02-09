//! Resolver backend trait — unified interface for all backends.
//!
//! PubGrub and Rez (and any future backend) implement [`Resolver`].
//! Selection is by config `plugins.pkg_rs.resolver_backend` (`"pkg"` | `"rez"`).

use crate::config;
use crate::error::SolverError;
use crate::package::Package;

/// Unified resolver interface.
///
/// All backends (PubGrub, Rez Python, future Rust Rez) use the same signature
/// so callers can switch backends without code changes.
pub trait Resolver: Send + Sync {
    /// Backend name for config and errors (e.g. `"pkg"`, `"rez"`).
    fn name(&self) -> &'static str;

    /// Resolve requirements into a list of concrete package names.
    ///
    /// * `packages` — available packages (PubGrub uses these; Rez may use config paths).
    /// * `requirements` — request strings (e.g. `["maya@>=2024", "redshift"]`).
    /// * `config` — optional Rez config (filters, orderers, paths).
    ///
    /// Returns sorted list of full package names (e.g. `["maya-2024.0.0", "redshift-3.5.0"]`).
    fn solve(
        &self,
        packages: &[Package],
        requirements: Vec<String>,
        config: Option<&config::Config>,
    ) -> Result<Vec<String>, SolverError>;
}
