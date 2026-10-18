use crate::{Error, Result};
use sha2::{Digest, Sha256};
use std::{io::Read, os::unix::fs::MetadataExt, path::Path};
pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn valid_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn parents(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Error("unsafe_path"));
    }
    for p in path.ancestors().skip(1) {
        let m = match std::fs::symlink_metadata(p) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(Error("unsafe_path")),
        };
        let owner = m.uid();
        let mode = m.mode();
        if !m.is_dir()
            || m.file_type().is_symlink()
            || (owner != 0 && owner != rustix::process::getuid().as_raw())
            || (mode & 0o022 != 0 && !(owner == 0 && mode & 0o1000 != 0))
        {
            return Err(Error("unsafe_path"));
        }
    }
    Ok(())
}
pub fn read(path: &Path, limit: usize, executable: bool) -> Result<Vec<u8>> {
    parents(path)?;
    let file = zixcel_bounded_io::open_regular_file(path).map_err(|_| Error("file_unavailable"))?;
    let m = file.metadata().map_err(|_| Error("file_unavailable"))?;
    if !m.is_file()
        || (m.uid() != 0 && m.uid() != rustix::process::getuid().as_raw())
        || m.mode() & 0o022 != 0
        || (executable && m.mode() & 0o111 == 0)
    {
        return Err(Error("unsafe_file"));
    }
    bounded(file, limit)
}
pub fn bounded(file: impl Read, limit: usize) -> Result<Vec<u8>> {
    zixcel_bounded_io::read_bounded(file, limit as u64).map_err(|e| {
        if e.get_ref()
            .is_some_and(|v| v.is::<zixcel_bounded_io::InputLimit>())
        {
            Error("file_too_large")
        } else {
            Error("file_unavailable")
        }
    })
}
