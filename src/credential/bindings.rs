use crate::{Error, Result};
use serde::Deserialize;
use std::{collections::HashSet, os::unix::fs::MetadataExt, path::Path};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub reference: String,
    pub host: String,
    pub environment: String,
    #[serde(default)]
    pub field: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    credentials: Vec<Binding>,
}
pub(super) fn read(path: &str) -> Result<Vec<Binding>> {
    let path = Path::new(path);
    crate::files::parents(path)?;
    if std::fs::canonicalize(path).map_err(|_| Error("credential_bindings_unavailable"))? != path {
        return Err(Error("unsafe_credential_bindings"));
    }
    let file = zixcel_bounded_io::open_regular_file(path)
        .map_err(|_| Error("credential_bindings_unavailable"))?;
    let m = file
        .metadata()
        .map_err(|_| Error("credential_bindings_unavailable"))?;
    if m.uid() != rustix::process::getuid().as_raw() || m.mode() & 0o077 != 0 {
        return Err(Error("unsafe_credential_bindings"));
    }
    let d: Document = serde_json::from_slice(&crate::files::bounded(file, 16384)?)
        .map_err(|_| Error("invalid_credential_bindings"))?;
    if d.schema != "zix-credential-bindings/v1"
        || d.credentials.is_empty()
        || d.credentials.len() > 8
    {
        return Err(Error("invalid_credential_bindings"));
    }
    let mut names = HashSet::new();
    for b in &d.credentials {
        super::options::validate_binding(b)?;
        if !names.insert(&b.environment) {
            return Err(Error("duplicate_credential_environment"));
        }
    }
    Ok(d.credentials)
}
