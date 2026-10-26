use crate::{Error, Result, files, registry::Registry};
use std::{
    fs::{DirBuilder, OpenOptions},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::Path,
};
pub fn mutate(path: &Path, operation: impl FnOnce(&mut Registry) -> Result<()>) -> Result<()> {
    files::parents(path)?;
    let parent = path.parent().ok_or(Error("unsafe_path"))?;
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)
        .map_err(|_| Error("registry_unavailable"))?;
    files::parents(path)?;
    let m = std::fs::symlink_metadata(parent).map_err(|_| Error("unsafe_registry"))?;
    if m.uid() != rustix::process::getuid().as_raw() || m.mode() & 0o077 != 0 {
        return Err(Error("unsafe_registry"));
    }
    let flags = rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(flags.bits() as i32)
        .open(parent.join(".registry.lock"))
        .map_err(|_| Error("registry_locked"))?;
    let m = lock.metadata().map_err(|_| Error("unsafe_registry"))?;
    if !m.is_file() || m.uid() != rustix::process::getuid().as_raw() || m.mode() & 0o077 != 0 {
        return Err(Error("unsafe_registry"));
    }
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| Error("registry_locked"))?;
    let mut registry = Registry::load(path)?;
    operation(&mut registry)?;
    registry.save(path)
}
mod candidate;
pub use candidate::{add, replace};
pub fn change(registry: &mut Registry, name: &str, operation: &str) -> Result<()> {
    let index = registry
        .entries
        .iter()
        .position(|e| e.manifest().is_ok_and(|m| m.namespace == name))
        .ok_or(Error("not_installed"))?;
    match operation {
        "enable" => {
            let entry = &registry.entries[index];
            if files::digest(&files::read(&entry.executable, 128 * 1024 * 1024, true)?)
                != entry.executable_sha256
            {
                return Err(Error("integrity_mismatch"));
            }
            registry.entries[index].enabled = true;
        }
        "disable" => registry.entries[index].enabled = false,
        "remove" => {
            registry.entries.remove(index);
        }
        _ => return Err(Error("unknown_operation")),
    }
    Ok(())
}
