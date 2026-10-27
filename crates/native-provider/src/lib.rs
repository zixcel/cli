#![forbid(unsafe_code)]
//! Shared protocol, policy validation and observations; no provider login or implicit grants.
mod binding;
pub mod bundle;
mod entry;
pub mod guidance;
mod host;
pub use host::{Failure, serve, serve_guided, validate_request};
pub use zix_cli::manifest::{Argument, Command, Manifest};
mod model;
mod operation_policy;
mod plan;
mod profile;
mod projection;
mod validation;
pub use entry::{entry, entry_guided};
pub use model::{Identity, Limits, Operation, Policy, Prepared, Projection, Request, Rule};
pub use profile::Provider;
pub use zix_cli::{Error, Result};
pub use zix_operation_contracts::guidance::Catalog as GuidanceCatalog;
