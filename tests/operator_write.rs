mod support;
use serde_json::{Value, json};
use support::Context;
fn candidate(c: &Context, schema: &str) -> Value {
    let mut m: Value = serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
    m["schema"] = json!(schema);
    m["commands"][0]["effect"] = json!("operator-write");
    m["commands"][0]["arguments"] = json!([
 {"name":"plan-file","required":true,"maximum":2048},
 {"name":"approval-sha256","required":true,"maximum":64},
 {"name":"journal-directory","required":true,"maximum":2048}]);
    m
}
#[test]
fn legacy_read_only_schema_cannot_smuggle_a_mutation() {
    let c = Context::new(1000);
    let m = candidate(&c, "zix-extension/v1");
    std::fs::write(&c.manifest, m.to_string()).unwrap();
    assert!(!c.install().status.success());
}
#[test]
fn write_manifest_needs_explicit_plan_hash_and_journal_and_unattended_execution_never_launches() {
    let c = Context::new(1000);
    let m = candidate(&c, "zix-extension/v2");
    std::fs::write(&c.manifest, m.to_string()).unwrap();
    assert!(c.install().status.success());
    let out = c.run(&[
        "sample",
        "inspect",
        "--plan-file",
        "/nonexistent/plan.json",
        "--approval-sha256",
        &"a".repeat(64),
        "--journal-directory",
        "/nonexistent/journal",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("interactive_authorization_required"));
    let mut invalid = m;
    invalid["commands"][0]["arguments"] = json!([]);
    std::fs::write(&c.manifest, invalid.to_string()).unwrap();
    assert!(!c.install().status.success());
}
