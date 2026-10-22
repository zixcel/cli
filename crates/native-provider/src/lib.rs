#![forbid(unsafe_code)]
//! Shared protocol, policy validation and observations; no provider login or implicit grants.
mod binding;
pub mod bundle;
mod entry;
mod host;
pub use host::{Failure, serve, validate_request};
pub use zix_cli::manifest::{Argument, Command, Manifest};
mod model;
mod operation_policy;
mod plan;
mod profile;
mod projection;
mod validation;
pub use entry::entry;
pub use model::{Identity, Limits, Operation, Policy, Prepared, Projection, Request, Rule};
pub use profile::Provider;
pub use zix_cli::{Error, Result};
