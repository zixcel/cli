mod support;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use support::Context;
struct Restore(PathBuf);
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
    }
}
#[test]
fn post_rename_sync_failure_reports_unknown_and_leaves_a_complete_inspectable_entry() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let parent = c.root.path().join("zix");
    let _restore = Restore(parent.clone());
    // Allow pathname traversal and rename, but deny read-open of the directory.
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o300)).unwrap();
    let out = c.run(&["extensions", "disable", "sample"]);
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("registry_commit_unknown"));
    let inspected = c.run(&["extensions", "inspect", "sample", "--format", "json"]);
    assert!(inspected.status.success());
    let entry: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(entry["entry"]["enabled"], false);
}
