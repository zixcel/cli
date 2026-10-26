mod support;
use serde_json::{Value, json};
use std::{fs, path::Path};
use support::Context;
fn hash(path: &Path) -> String {
    zix_cli::files::digest(&fs::read(path).unwrap())
}
fn inspect(c: &Context) -> Value {
    let out = c.run(&["extensions", "inspect", "sample", "--format", "json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice::<Value>(&out.stdout).unwrap()["entry"].clone()
}
fn replace(c: &Context, old: &Value) -> std::process::Output {
    c.run(&[
        "extensions",
        "replace",
        "--manifest",
        c.manifest.to_str().unwrap(),
        "--manifest-sha256",
        &hash(&c.manifest),
        "--executable",
        c.executable.to_str().unwrap(),
        "--executable-sha256",
        &hash(&c.executable),
        "--expected-manifest-sha256",
        old["manifest_sha256"].as_str().unwrap(),
        "--expected-executable-sha256",
        old["executable_sha256"].as_str().unwrap(),
    ])
}
fn version(c: &Context, version: &str) {
    let mut m: Value = serde_json::from_slice(&fs::read(&c.manifest).unwrap()).unwrap();
    m["version"] = json!(version);
    fs::write(&c.manifest, m.to_string()).unwrap();
}
#[test]
fn disabled_registration_is_inspectable_updatable_and_reversible_without_activation() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    assert!(c.run(&["extensions", "disable", "sample"]).status.success());
    let old = inspect(&c);
    let list = c.run(&["extensions", "list", "--format", "json"]);
    let list: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(list["entries"][0]["enabled"], false);
    assert!(
        !c.run(&["sample", "inspect", "--mode", "echo"])
            .status
            .success()
    );
    version(&c, "0.2.0");
    assert!(replace(&c, &old).status.success());
    let upgraded = inspect(&c);
    assert_eq!(upgraded["version"], "0.2.0");
    assert_eq!(upgraded["enabled"], false);
    version(&c, "0.1.0");
    assert!(replace(&c, &upgraded).status.success());
    assert_eq!(inspect(&c)["version"], "0.1.0");
    fs::remove_file(&c.executable).unwrap();
    assert_eq!(inspect(&c)["enabled"], false);
}
#[test]
fn stale_replacement_or_package_change_preserves_current_registration_byte_for_byte() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let old = inspect(&c);
    version(&c, "0.2.0");
    assert!(replace(&c, &old).status.success());
    let upgraded = inspect(&c);
    let path = c.root.path().join("zix/extensions.json");
    let before = fs::read(&path).unwrap();
    version(&c, "0.3.0");
    let rejected = replace(&c, &old);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("registration_changed"));
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut m: Value = serde_json::from_slice(&fs::read(&c.manifest).unwrap()).unwrap();
    m["package"] = json!("another-package");
    fs::write(&c.manifest, m.to_string()).unwrap();
    let rejected = replace(&c, &upgraded);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("package_identity_mismatch"));
    assert_eq!(fs::read(&path).unwrap(), before);
}
#[test]
fn replacement_verification_failure_keeps_old_provider_available() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let old = inspect(&c);
    let path = c.root.path().join("zix/extensions.json");
    let before = fs::read(&path).unwrap();
    version(&c, "0.2.0");
    let rejected = c.run(&[
        "extensions",
        "replace",
        "--manifest",
        c.manifest.to_str().unwrap(),
        "--manifest-sha256",
        &"0".repeat(64),
        "--executable",
        c.executable.to_str().unwrap(),
        "--executable-sha256",
        &hash(&c.executable),
        "--expected-manifest-sha256",
        old["manifest_sha256"].as_str().unwrap(),
        "--expected-executable-sha256",
        old["executable_sha256"].as_str().unwrap(),
    ]);
    assert!(!rejected.status.success());
    assert_eq!(fs::read(path).unwrap(), before);
    assert!(
        c.run(&["sample", "inspect", "--mode", "echo"])
            .status
            .success()
    );
}
#[test]
fn replacement_obeys_same_mutation_lock() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let old = inspect(&c);
    let lock = fs::File::open(c.root.path().join("zix/.registry.lock")).unwrap();
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::LockExclusive).unwrap();
    let rejected = replace(&c, &old);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("registry_locked"));
}
