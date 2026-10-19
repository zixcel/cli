mod support;
use serde_json::{Value, json};
use std::fs;
use support::Context;

fn declare(context: &Context, names: Value) {
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&context.manifest).unwrap()).unwrap();
    manifest["runtime_environment"] = names;
    fs::write(&context.manifest, manifest.to_string()).unwrap();
}

#[test]
fn session_bus_is_opt_in_and_not_an_arbitrary_environment_escape() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let run = || {
        c.command()
            .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/tmp/synthetic-bus")
            .args(["sample", "inspect", "--mode", "echo"])
            .output()
            .unwrap()
    };
    let value: Value = serde_json::from_slice(&run().stdout).unwrap();
    assert_eq!(value["result"]["session_bus"], false);
    assert!(c.run(&["extensions", "remove", "sample"]).status.success());
    declare(&c, json!(["DBUS_SESSION_BUS_ADDRESS"]));
    assert!(c.install().status.success());
    let value: Value = serde_json::from_slice(&run().stdout).unwrap();
    assert_eq!(value["result"]["session_bus"], true);
    for names in [
        json!(["PATH"]),
        json!(["LD_PRELOAD"]),
        json!(["DBUS_SESSION_BUS_ADDRESS", "DBUS_SESSION_BUS_ADDRESS"]),
    ] {
        declare(&c, names);
        assert!(!c.install().status.success());
    }
}

#[test]
fn missing_or_remote_bus_is_rejected_before_child_execution() {
    let c = Context::new(5000);
    declare(&c, json!(["DBUS_SESSION_BUS_ADDRESS"]));
    assert!(c.install().status.success());
    for address in [
        None,
        Some("tcp:host=example.test,port=1234"),
        Some("unix:path=/tmp/a;tcp:host=example.test"),
    ] {
        let mut command = c.command();
        if let Some(address) = address {
            command.env("DBUS_SESSION_BUS_ADDRESS", address);
        }
        let out = command
            .args(["sample", "inspect", "--mode", "echo"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
}
