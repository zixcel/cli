use crate::{Error, Result, files, registry::Entry};
use std::{fs::File, io::Write, os::fd::AsRawFd, path::PathBuf};
pub struct Executable {
    pub file: File,
    pub path: PathBuf,
}
pub fn snapshot(entry: &Entry) -> Result<Executable> {
    let bytes = files::read(&entry.executable, 128 * 1024 * 1024, true)?;
    if files::digest(&bytes) != entry.executable_sha256 || !bytes.starts_with(b"\x7fELF") {
        return Err(Error("integrity_mismatch"));
    }
    let fd = rustix::fs::memfd_create(
        "zix-verified-provider",
        rustix::fs::MemfdFlags::CLOEXEC | rustix::fs::MemfdFlags::ALLOW_SEALING,
    )
    .map_err(|_| Error("sealed_execution_unavailable"))?;
    let mut file = File::from(fd);
    file.write_all(&bytes)
        .map_err(|_| Error("sealed_execution_unavailable"))?;
    use rustix::fs::SealFlags;
    rustix::fs::fcntl_add_seals(
        &file,
        SealFlags::SEAL | SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK,
    )
    .map_err(|_| Error("sealed_execution_unavailable"))?;
    let path = PathBuf::from(format!(
        "/proc/{}/fd/{}",
        std::process::id(),
        file.as_raw_fd()
    ));
    Ok(Executable { file, path })
}
