use crate::{Error, Result, files, registry::Registry};
use std::{
    fs::{DirBuilder, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::Path,
};
pub(crate) fn save(registry: &Registry, path: &Path) -> Result<()> {
    files::parents(path)?;
    let parent = path.parent().ok_or(Error("unsafe_path"))?;
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)
        .map_err(|_| Error("registry_unavailable"))?;
    files::parents(path)?;
    let m = std::fs::symlink_metadata(parent).map_err(|_| Error("registry_unavailable"))?;
    if m.uid() != rustix::process::getuid().as_raw() || m.mode() & 0o077 != 0 {
        return Err(Error("unsafe_registry"));
    }
    let bytes = serde_json::to_vec(registry).map_err(|_| Error("invalid_registry"))?;
    if bytes.len() > 65536 {
        return Err(Error("registry_too_large"));
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error("clock_unavailable"))?
        .as_nanos();
    let temporary = parent.join(format!(".extensions-{}-{stamp}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| Error("registry_unavailable"))?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        Ok::<_, std::io::Error>(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
        return Err(Error("registry_unavailable"));
    }
    Ok(())
}
