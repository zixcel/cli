mod support;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use support::Context;
use zix_cli::{manifest::Manifest, registry::Registry};
#[test]
fn manifest_rejects_incompatible_unknown_conflicting_and_privileged_fields() {
    let c = Context::new(5000);
    let original: Value = serde_json::from_slice(&fs::read(&c.manifest).unwrap()).unwrap();
    for (field, value) in [
        ("schema", json!("unknown")),
        ("namespace", json!("extensions")),
        ("namespace", json!("api")),
        ("platform", json!("windows-x86_64")),
        ("credential_environment", json!(["LD_PRELOAD"])),
        ("timeout_ms", json!(120001)),
        ("unknown", json!(true)),
    ] {
        let mut m = original.clone();
        m[field] = value;
        assert!(Manifest::parse(&serde_json::to_vec(&m).unwrap()).is_err());
    }
    let mut m = original.clone();
    m["commands"][0]["effect"] = json!("write");
    assert!(Manifest::parse(&serde_json::to_vec(&m).unwrap()).is_err());
    let mut m = original;
    let duplicate = m["commands"][0].clone();
    m["commands"].as_array_mut().unwrap().push(duplicate);
    assert!(Manifest::parse(&serde_json::to_vec(&m).unwrap()).is_err());
}
#[test]
fn unknown_duplicate_missing_and_control_arguments_never_echo_values() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    for args in [
        vec!["sample", "inspect"],
        vec!["sample", "inspect", "--unknown", "PRIVATE_MARKER"],
        vec!["sample", "inspect", "--mode", "echo", "--mode", "echo"],
        vec!["sample", "inspect", "--mode", "echo\nPRIVATE_MARKER"],
    ] {
        let out = c.run(&args);
        assert!(!out.status.success());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("PRIVATE_MARKER"));
        assert!(out.stdout.is_empty());
    }
}
#[test]
fn symlink_manifest_parent_or_executable_is_not_admitted() {
    let c = Context::new(5000);
    let original = c.manifest.with_extension("original");
    fs::rename(&c.manifest, &original).unwrap();
    symlink(&original, &c.manifest).unwrap();
    assert!(!c.install().status.success());
    fs::remove_file(&c.manifest).unwrap();
    fs::rename(original, &c.manifest).unwrap();
    let original = c.executable.with_extension("original");
    fs::rename(&c.executable, &original).unwrap();
    symlink(&original, &c.executable).unwrap();
    assert!(!c.install().status.success());
}
#[test]
fn namespace_collision_registry_tampering_and_public_registry_modes_are_rejected() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    assert!(!c.install().status.success());
    let path = c.root.path().join("zix/extensions.json");
    let bytes = fs::read(&path).unwrap();
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["entries"][0]["manifest_text"] = json!("{}");
    fs::write(&path, value.to_string()).unwrap();
    assert!(Registry::load(&path).is_err());
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Registry::load(&path).is_err());
}
#[test]
fn mutation_lock_prevents_lost_updates_and_does_not_enable_partial_install() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let p = c.root.path().join("zix/.registry.lock");
    let lock = fs::File::open(p).unwrap();
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::LockExclusive).unwrap();
    let out = c.run(&["extensions", "disable", "sample"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("registry_locked"));
    drop(lock);
    assert!(c.run(&["extensions", "disable", "sample"]).status.success());
    assert!(
        !c.run(&["sample", "inspect", "--mode", "echo"])
            .status
            .success()
    );
}
#[test]
fn missing_and_dangling_registry_paths_are_different() {
    let c = Context::new(5000);
    let parent = c.root.path().join("zix");
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    let path = parent.join("extensions.json");
    assert!(Registry::load(&path).unwrap().entries.is_empty());
    symlink(parent.join("missing.json"), &path).unwrap();
    assert!(Registry::load(&path).is_err());
}
