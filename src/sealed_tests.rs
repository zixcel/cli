use crate::{files, registry::Entry, sealed};
use std::{
    fs,
    io::Write,
    os::unix::fs::{FileExt, PermissionsExt},
};
#[test]
fn verified_execution_copy_cannot_be_changed_or_retargeted() {
    let root = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let executable = root.path().join("provider");
    fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let bytes = fs::read(&executable).unwrap();
    let entry = Entry {
        enabled: true,
        manifest_text: String::new(),
        manifest_sha256: String::new(),
        executable: executable.clone(),
        executable_sha256: files::digest(&bytes),
    };
    let mut copy = sealed::snapshot(&entry).unwrap();
    fs::write(&executable, b"replacement").unwrap();
    assert!(copy.file.write_all(b"tamper").is_err());
    let mut header = [0; 4];
    copy.file.read_exact_at(&mut header, 0).unwrap();
    assert_eq!(&header, b"\x7fELF");
    let flags = rustix::fs::fcntl_get_seals(&copy.file).unwrap();
    assert!(flags.contains(rustix::fs::SealFlags::WRITE | rustix::fs::SealFlags::SEAL));
}
