use crate::{Error, Result, files, manifest::Manifest};
use serde::{Deserialize, Serialize};
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub enabled: bool,
    pub manifest_text: String,
    pub manifest_sha256: String,
    pub executable: PathBuf,
    pub executable_sha256: String,
}
impl Entry {
    pub fn manifest(&self) -> Result<Manifest> {
        if files::digest(self.manifest_text.as_bytes()) != self.manifest_sha256
            || !files::valid_digest(&self.executable_sha256)
        {
            return Err(Error("integrity_mismatch"));
        }
        Manifest::parse(self.manifest_text.as_bytes())
    }
}
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub version: u8,
    pub entries: Vec<Entry>,
}
pub fn location() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .ok_or(Error("config_home_missing"))?;
    if !base.is_absolute() {
        return Err(Error("unsafe_path"));
    }
    Ok(base.join("zix/extensions.json"))
}
impl Registry {
    pub fn load(path: &Path) -> Result<Self> {
        files::parents(path)?;
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    version: 1,
                    entries: vec![],
                });
            }
            Err(_) => return Err(Error("registry_unavailable")),
        };
        if metadata.file_type().is_symlink()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.mode() & 0o077 != 0
        {
            return Err(Error("unsafe_registry"));
        }
        let r: Self = serde_json::from_slice(&files::read(path, 65536, false)?)
            .map_err(|_| Error("invalid_registry"))?;
        if r.version != 1 || r.entries.len() > 32 {
            return Err(Error("invalid_registry"));
        }
        let mut names = std::collections::HashSet::new();
        for e in &r.entries {
            if !names.insert(e.manifest()?.namespace) {
                return Err(Error("namespace_collision"));
            }
        }
        Ok(r)
    }
    pub fn entry(&self, name: &str) -> Result<&Entry> {
        let e = self
            .entries
            .iter()
            .find(|e| e.manifest().is_ok_and(|m| m.namespace == name))
            .ok_or(Error("not_installed"))?;
        if !e.enabled {
            return Err(Error("disabled"));
        }
        Ok(e)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        crate::registry_store::save(self, path)
    }
}
