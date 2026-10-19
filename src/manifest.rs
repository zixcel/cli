use crate::{Error, Result};
use std::collections::HashSet;
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
        let mut ids = HashSet::new();
        let mut commands = HashSet::new();
        let mut env = HashSet::new();
        if self.schema != "zix-extension/v1"
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
        for c in &self.commands {
            let mut args = HashSet::new();
            if !clean(&c.id, 100)
                || !ids.insert(&c.id)
                || c.effect != "read"
                || !clean(&c.description, 160)
                || c.words.is_empty()
                || c.words.len() > 6
                || c.words.iter().any(|w| !token(w))
                || !commands.insert(&c.words)
                || c.arguments.len() > 16
                || c.required_permissions.len() > 16
                || c.required_permissions.iter().any(|p| !clean(p, 128))
            {
                return Err(Error("invalid_manifest"));
            }
            for a in &c.arguments {
                if !token(&a.name) || !args.insert(&a.name) || !(1..=2048).contains(&a.maximum) {
                    return Err(Error("invalid_manifest"));
                }
            }
        }
        for (i, a) in self.commands.iter().enumerate() {
            if self
                .commands
                .iter()
                .skip(i + 1)
                .any(|b| a.words.starts_with(&b.words) || b.words.starts_with(&a.words))
            {
                return Err(Error("ambiguous_operation"));
            }
        }
        Ok(())
    }
}
