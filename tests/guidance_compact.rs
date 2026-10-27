mod support;
use serde_json::{Value, json};
fn guided() -> support::Context {
    let c = support::Context::new(5000);
    let mut m: Value = serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
    m["protocol"] = json!("zix-provider/v2");
    std::fs::write(&c.manifest, m.to_string()).unwrap();
    assert!(c.install().status.success());
    c
}
#[test]
fn compact_index_and_selected_operation_retain_executable_shape_and_context() {
    let c = guided();
    let o = c.run(&["capabilities", "--compact", "--format", "json"]);
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["schema"], "zix-capability-index/v1");
    assert_eq!(v["remote_requests"], false);
    assert!(v["providers"][0].get("credential_environment").is_none());
    assert!(v["providers"][0].get("operations").is_none());
    let o = c.run(&[
        "capabilities",
        "--namespace",
        "sample",
        "--compact",
        "--format",
        "json",
    ]);
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["scope"], "operations");
    assert_eq!(v["providers"][0]["operations"][0]["id"], "sample.inspect");
    assert!(
        !c.run(&["capabilities", "--namespace", "missing", "--compact"])
            .status
            .success()
    );
    let o = c.run(&[
        "describe",
        "sample",
        "--operation",
        "sample.inspect",
        "--argument",
        "mode=needs-context",
        "--compact",
        "--format",
        "json",
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["command"], json!(["zix", "sample", "inspect"]));
    assert_eq!(v["missing_arguments"], json!(["value"]));
    assert_eq!(v["selected_arguments"], json!({"mode":"needs-context"}));
    assert_eq!(v["selector_choices"]["mode"], json!(["needs-context"]));
    assert_eq!(v["requirements_state"], "declared_not_verified");
    assert_eq!(v["authority"], "none");
    assert!(
        !c.run(&["describe", "sample", "--guidance", "--compact"])
            .status
            .success()
    );
}
#[test]
fn conditional_missing_input_stops_before_provider_and_credential_store() {
    let c = guided();
    let o = c
        .command()
        .env("ZIX_TEST_ACCESS_TOKEN", "SYNTHETIC_PRIVATE")
        .args(["sample", "inspect", "--mode", "needs-context"])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(v["error"], "operation_context_missing");
    assert!(
        v["diagnostic"]["inspect_argv"]
            .as_array()
            .unwrap()
            .contains(&json!("mode=needs-context"))
    );
    #[cfg(feature = "credential-store")]
    {
        let hash = |p: &std::path::Path| zix_cli::files::digest(&std::fs::read(p).unwrap());
        let o = c.run(&[
            "credential",
            "run",
            "--collection",
            "default",
            "--reference",
            "local/sample/token/sample/default",
            "--host",
            "service.example.invalid",
            "--environment",
            "ZIX_TEST_ACCESS_TOKEN",
            "--manifest-sha256",
            &hash(&c.manifest),
            "--executable-sha256",
            &hash(&c.executable),
            "--",
            "sample",
            "inspect",
            "--mode",
            "needs-context",
        ]);
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(o.status.code(), Some(2));
        assert_eq!(v["error"], "operation_context_missing");
    }
}

#[test]
fn compact_known_inputs_are_not_reported_missing() {
    let c = guided();
    let o = c.run(&[
        "describe",
        "sample",
        "--operation",
        "sample.inspect",
        "--argument",
        "mode=needs-context",
        "--argument",
        "value=known",
        "--compact",
        "--format",
        "json",
    ]);
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["selected_arguments"]["value"], "known");
    assert_eq!(v["missing_arguments"], json!([]));
    assert!(
        v["inspect_argv"]
            .as_array()
            .unwrap()
            .contains(&json!("value=known"))
    );
}
