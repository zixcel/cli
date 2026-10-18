mod support;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use support::Context;
#[test]
fn symlink_ancestor_and_nonregular_or_oversized_inputs_are_rejected() {
    let c = Context::new(5000);
    let directory = c.executable.parent().unwrap();
    let moved = c.root.path().join("moved-provider");
    fs::rename(directory, &moved).unwrap();
    symlink(&moved, directory).unwrap();
    assert!(!c.install().status.success());
    let fifo = c.root.path().join("fifo");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    let start = std::time::Instant::now();
    assert!(zix_cli::files::read(&fifo, 1024, false).is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    let large = c.root.path().join("large");
    fs::write(&large, vec![0; 1025]).unwrap();
    fs::set_permissions(&large, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        zix_cli::files::read(&large, 1024, false).unwrap_err().0,
        "file_too_large"
    );
}
