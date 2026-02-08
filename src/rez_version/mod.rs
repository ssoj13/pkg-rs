//! Rez-compatible versioning primitives.
//!
//! Implements Rez's alphanumeric version semantics and version range parsing.

mod range;
mod requirement;
mod version;

pub use range::{Bound, LowerBound, UpperBound, VersionRange};
pub use requirement::Requirement;
pub use version::{SubToken, Token, Version, VersionError};
