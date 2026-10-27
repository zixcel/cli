mod support;
use serde_json::json;
fn guided() -> support::Context {
    let c = support::Context::new(5000);
    let mut m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
    m["protocol"] = json!("zix-provider/v2");
    std::fs::write(&c.manifest, m.to_string()).unwrap();
    assert!(c.install().status.success());
    c
}
#[test]
fn llm_discovery_is_credential_free_and_does_not_invoke_the_operation() {
    let c = guided();
    let o = c
        .command()
        .env("ZIX_TEST_ACCESS_TOKEN", "SYNTHETIC_PRIVATE")
        .args([
            "describe",
            "sample",
            "--operation",
            "sample.inspect",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["remote_requests"], false);
    assert_eq!(v["credentials_read"], false);
    assert_eq!(v["guide"]["intent"], "inspect_sample");
    assert_eq!(
        v["resolved_actions"][0]["authorization_state"],
        "not_inferred"
    );
    assert!(
        !c.run(&[
            "describe",
            "sample",
            "--operation",
            "sample.inspect",
            "--argument",
            "invented=value"
        ])
        .status
        .success()
    );
}
#[test]
fn denied_partial_and_unknown_results_preserve_evidence_and_never_retry() {
    let c = guided();
    let count = c.root.path().join("calls");
    for mode in ["v2-denied", "v2-incomplete", "v2-unknown"] {
        let o = c.run(&[
            "sample",
            "inspect",
            "--mode",
            mode,
            "--value",
            count.to_str().unwrap(),
        ]);
        assert_eq!(
            o.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["diagnostic"]["state"], "NeedsResolution");
        assert_eq!(v["diagnostic"]["retry"], "no_automatic_retry");
        assert_eq!(
            v["diagnostic"]["resolved_actions"][0]["authorization_state"],
            "not_inferred"
        );
    }
    assert_eq!(std::fs::read_to_string(count).unwrap().lines().count(), 3);
    assert!(
        !c.run(&["sample", "inspect", "--mode", "v2-invalid"])
            .status
            .success()
    );
}
