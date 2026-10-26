use crate::{
    Error, Result, files,
    manifest::Manifest,
    registry::{Entry, Registry},
};
use std::path::Path;
fn candidate(
    manifest_path: &Path,
    manifest_sha: &str,
    executable: &Path,
    executable_sha: &str,
) -> Result<Entry> {
    if !files::valid_digest(manifest_sha) || !files::valid_digest(executable_sha) {
        return Err(Error("invalid_digest"));
    }
    let bytes = files::read(manifest_path, 16384, false)?;
    if files::digest(&bytes) != manifest_sha {
        return Err(Error("integrity_mismatch"));
    }
    Manifest::parse(&bytes)?;
    let binary = files::read(executable, 128 * 1024 * 1024, true)?;
    if files::digest(&binary) != executable_sha || !binary.starts_with(b"\x7fELF") {
        return Err(Error("integrity_mismatch"));
    }
    Ok(Entry {
        enabled: true,
        manifest_text: String::from_utf8(bytes).map_err(|_| Error("invalid_manifest"))?,
        manifest_sha256: manifest_sha.into(),
        executable: executable.into(),
        executable_sha256: executable_sha.into(),
    })
}
pub fn add(
    registry: &mut Registry,
    manifest_path: &Path,
    manifest_sha: &str,
    executable: &Path,
    executable_sha: &str,
) -> Result<()> {
    let entry = candidate(manifest_path, manifest_sha, executable, executable_sha)?;
    let namespace = entry.manifest()?.namespace;
    if registry.entries.len() >= 32
        || registry
            .entries
            .iter()
            .any(|e| e.manifest().is_ok_and(|m| m.namespace == namespace))
    {
        return Err(Error("namespace_collision"));
    }
    registry.entries.push(entry);
    Ok(())
}
/// Called under the same registry lock as candidate verification and atomic save.
/// Explicit expected hashes prevent stale operators from replacing a newer entry.
pub fn replace(
    registry: &mut Registry,
    manifest_path: &Path,
    manifest_sha: &str,
    executable: &Path,
    executable_sha: &str,
    expected_manifest: &str,
    expected_executable: &str,
) -> Result<()> {
    if !files::valid_digest(expected_manifest) || !files::valid_digest(expected_executable) {
        return Err(Error("invalid_digest"));
    }
    let mut entry = candidate(manifest_path, manifest_sha, executable, executable_sha)?;
    let proposed = entry.manifest()?;
    let old = registry.registered(&proposed.namespace)?;
    if old.manifest_sha256 != expected_manifest || old.executable_sha256 != expected_executable {
        return Err(Error("registration_changed"));
    }
    if old.manifest()?.package != proposed.package {
        return Err(Error("package_identity_mismatch"));
    }
    entry.enabled = old.enabled;
    let index = registry
        .entries
        .iter()
        .position(|e| {
            e.manifest()
                .is_ok_and(|m| m.namespace == proposed.namespace)
        })
        .ok_or(Error("not_installed"))?;
    registry.entries[index] = entry;
    Ok(())
}
