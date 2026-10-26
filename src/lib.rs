#![forbid(unsafe_code)]
//! Linux native extensions are trusted executable code, never automatically discovered or installed.
mod api_command;
pub mod api_spec;
mod arguments;
pub mod command_line;
pub mod discovery;
mod error;
pub mod execution;
mod extension_inventory;
mod extensions;
pub mod files;
pub mod install;
pub mod manifest;
mod operator;
pub mod registry;
pub mod release_bundle;
mod sealed;
pub use arguments::{Selected, select};
pub use error::{Error, Result};

mod registry_store;
pub mod response;
#[cfg(test)]
mod sealed_tests;
