mod support;
use serde_json::Value;
use std::fs;
use support::Context;
#[test]
fn declared_opaque_cursor_budget_is_preserved_and_excess_is_rejected() {
    let c = Context::new(5000);
    let mut manifest: Value = serde_json::from_slice(&fs::read(&c.manifest).unwrap()).unwrap();
    manifest["commands"][0]["arguments"][1]["maximum"] = serde_json::json!(2048);
    fs::write(&c.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(c.install().status.success());
    let cursor = "x".repeat(2048);
    let out = c.run(&["sample", "inspect", "--mode", "echo", "--value", &cursor]);
    assert!(out.status.success());
    let response: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(response["result"]["value"], cursor);
    let excess = "x".repeat(2049);
    let out = c.run(&["sample", "inspect", "--mode", "echo", "--value", &excess]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&out.stderr).contains(&excess));
}
