mod support;
use serde_json::{Value, json};
use std::fs;
use support::Context;
fn declare(c: &Context, scope: Value) {
    let mut manifest: Value = serde_json::from_slice(&fs::read(&c.manifest).unwrap()).unwrap();
    manifest["runtime_environment"] = json!(["DBUS_SESSION_BUS_ADDRESS"]);
    manifest["commands"][0]["environment"] = scope;
    fs::write(&c.manifest, manifest.to_string()).unwrap();
}
#[test]
fn command_owned_empty_environment_neither_requires_nor_forwards_parent_custody() {
    let c = Context::new(5000);
    declare(
        &c,
        json!({"credential_environment":[],"runtime_environment":[]}),
    );
    assert!(c.install().status.success());
    for bus in [
        None,
        Some("unix:path=/tmp/synthetic-bus"),
        Some("tcp:host=invalid"),
    ] {
        let mut command = c.command();
        command.env("ZIX_TEST_ACCESS_TOKEN", "SYNTHETIC_TOKEN");
        if let Some(value) = bus {
            command.env("DBUS_SESSION_BUS_ADDRESS", value);
        }
        let out = command
            .args(["sample", "inspect", "--mode", "echo"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["result"]["session_bus"], false);
        assert_eq!(value["result"]["selected_environment"], false);
        assert!(!String::from_utf8_lossy(&out.stdout).contains("SYNTHETIC_TOKEN"));
    }
}
#[test]
fn subset_cannot_add_environment_or_duplicate_names() {
    for scope in [
        json!({"credential_environment":["ZIX_UNDECLARED_ACCESS_TOKEN"],"runtime_environment":[]}),
        json!({"credential_environment":[],"runtime_environment":["PATH"]}),
        json!({"credential_environment":[],"runtime_environment":["DBUS_SESSION_BUS_ADDRESS","DBUS_SESSION_BUS_ADDRESS"]}),
        json!({"credential_environment":["ZIX_TEST_ACCESS_TOKEN","ZIX_TEST_ACCESS_TOKEN"],"runtime_environment":[]}),
    ] {
        let c = Context::new(5000);
        declare(&c, scope);
        assert!(!c.install().status.success());
    }
}
#[test]
fn selected_credential_subset_keeps_redaction_and_does_not_require_the_bus() {
    let c = Context::new(5000);
    declare(
        &c,
        json!({"credential_environment":["ZIX_TEST_ACCESS_TOKEN"],"runtime_environment":[]}),
    );
    assert!(c.install().status.success());
    let out = c
        .command()
        .env("ZIX_TEST_ACCESS_TOKEN", "SYNTHETIC_TOKEN")
        .args(["sample", "inspect", "--mode", "echo"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["result"]["selected_environment"], true);
    assert_eq!(value["result"]["session_bus"], false);
    let out = c
        .command()
        .env("ZIX_TEST_ACCESS_TOKEN", "SYNTHETIC_TOKEN")
        .args(["sample", "inspect", "--mode", "secret"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!String::from_utf8_lossy(&out.stdout).contains("SYNTHETIC_TOKEN"));
}
