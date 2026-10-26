mod support;
use support::Context;
#[test]
fn empty_inventory_and_help_are_read_only_and_document_replacement() {
    let c = Context::new(1000);
    let out = c.run(&["extensions", "list", "--format", "json"]);
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["entries"], serde_json::json!([]));
    let out = c.run(&["extensions", "--help"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("--expected-manifest-sha256"));
    assert!(!c.root.path().join("zix/extensions.json").exists());
    assert!(!c.run(&["extensions", "inspect", "sample"]).status.success());
    assert!(!c.root.path().join("zix/extensions.json").exists());
    assert!(c.install().status.success());
    let out = c.run(&["extensions", "inspect", "sample", "--format", "json"]);
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["entry"]["namespace"], "sample");
    assert_eq!(value["entry"]["enabled"], true);
}
