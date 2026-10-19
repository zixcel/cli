use crate::{Error, Result};
use std::collections::HashSet;
mod commands;
mod model;
pub use model::{Argument, Command, Manifest};
pub fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && !s.ends_with('-')
}
fn clean(s: &str, limit: usize) -> bool {
    !s.is_empty() && s.len() <= limit && s.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
}
impl Manifest {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 16384 {
            return Err(Error("manifest_too_large"));
        }
        let m: Self = serde_json::from_slice(bytes).map_err(|_| Error("invalid_manifest"))?;
        m.validate()?;
        Ok(m)
    }
    pub fn validate(&self) -> Result<()> {
        let mut env = HashSet::new();
        if !["zix-extension/v1", "zix-extension/v2"].contains(&self.schema.as_str())
            || self.protocol != "zix-provider/v1"
            || !token(&self.namespace)
            || [
                "api",
                "help",
                "list",
                "capabilities",
                "describe",
                "prompt",
                "extensions",
                "version",
            ]
            .contains(&self.namespace.as_str())
            || !clean(&self.package, 128)
            || !clean(&self.version, 64)
            || self.platform != "linux-x86_64"
            || std::env::consts::OS != "linux"
            || std::env::consts::ARCH != "x86_64"
            || self.commands.is_empty()
            || self.commands.len() > 32
            || self.credential_environment.len() > 8
            || self.runtime_environment.len() > 1
            || !(50..=40000).contains(&self.timeout_ms)
        {
            return Err(Error("invalid_manifest"));
        }
        if self
            .runtime_environment
            .iter()
            .any(|name| name != "DBUS_SESSION_BUS_ADDRESS")
        {
            return Err(Error("invalid_runtime_environment"));
        }
        for name in &self.credential_environment {
            if !name.starts_with("ZIX_")
                || name.len() > 80
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                || !env.insert(name)
            {
                return Err(Error("invalid_manifest"));
            }
        }
        commands::validate(self)?;
        Ok(())
    }
}
