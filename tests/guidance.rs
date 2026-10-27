mod support;
use serde_json::json;
use zix_cli::{guidance, registry::Registry};
use zix_operation_contracts::guidance::Action;
fn action() -> Action {
    serde_json::from_value(json!({"namespace":"sample","operation":"sample.inspect","purpose":"inspect_evidence","arguments":{},"on":["authorization"]})).unwrap()
}
fn registry() -> (support::Context, Registry) {
    let c = support::Context::new(5000);
    assert!(c.install().status.success());
    let r = Registry::load(&c.root.path().join("zix/extensions.json")).unwrap();
    (c, r)
}
#[test]
fn actions_resolve_only_registered_commands_and_expose_missing_context_without_execution() {
    let (_c, mut r) = registry();
    let mut a = action();
    let v = guidance::resolve(&r, &a).unwrap();
    assert_eq!(v["availability"], "installed");
    assert_eq!(v["missing_arguments"], json!(["mode"]));
    assert_eq!(v["runnable"], false);
    assert_eq!(v["authorization_state"], "not_inferred");
    a.arguments.insert("mode".into(), "echo".into());
    let v = guidance::resolve(&r, &a).unwrap();
    assert_eq!(
        v["argv"],
        json!(["zix", "sample", "inspect", "--mode", "echo"])
    );
    assert_eq!(v["runnable"], true);
    a.requires_context
        .insert("authoritative_dns_provider".into(), "cloudflare".into());
    let v = guidance::resolve(&r, &a).unwrap();
    assert_eq!(v["runnable"], false);
    assert_eq!(v["context_state"], "unverified");
    a.requires_context.clear();
    r.entries[0].enabled = false;
    assert_eq!(guidance::resolve(&r, &a).unwrap()["runnable"], false);
    a.namespace = "missing".into();
    assert_eq!(
        guidance::resolve(&r, &a).unwrap()["availability"],
        "not_installed"
    );
    a.namespace = "sample".into();
    a.operation = "fabricated".into();
    assert_eq!(
        guidance::resolve(&r, &a).unwrap()["availability"],
        "unsupported_operation"
    );
}
#[test]
fn stale_arguments_and_unknown_write_failures_do_not_become_retry_instructions() {
    let (_c, r) = registry();
    let mut a = action();
    a.arguments.insert("invented".into(), "value".into());
    assert_eq!(
        guidance::resolve(&r, &a).unwrap()["availability"],
        "incompatible_arguments"
    );
    let d = guidance::local_failure("operator_write_outcome_unknown");
    assert_eq!(d["diagnostic"]["reason"], "OutcomeUnknown");
    assert_eq!(d["diagnostic"]["retry"], "no_automatic_retry");
}
#[test]
fn credential_help_resolves_as_a_builtin_without_secret_or_remote_access() {
    let mut a = action();
    a.namespace = "credential".into();
    a.operation = "help".into();
    let v = guidance::resolve(&Registry::default(), &a).unwrap();
    assert_eq!(v["argv"], json!(["zix", "credential", "--help"]));
    assert_eq!(v["credentials_read"], false);
    assert_eq!(v["remote_requests"], false);
    a.operation = "set".into();
    assert_eq!(
        guidance::resolve(&Registry::default(), &a).unwrap()["availability"],
        "unsupported_builtin"
    );
}
#[test]
fn legacy_provider_guidance_fails_locally_and_description_without_namespace_never_panics() {
    let (c, _) = registry();
    let o = c.run(&[
        "describe",
        "sample",
        "--operation",
        "sample.inspect",
        "--format",
        "json",
    ]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("guidance_not_supported"));
    assert!(!c.run(&["describe"]).status.success());
}
#[test]
fn diagnostic_inspection_preserves_only_declared_selectors() {
    let (_c, r) = registry();
    let mut response = json!({"error":"permission_denied","diagnostic":{
        "schema":"zix-diagnostic/v1","state":"NeedsResolution","reason":"authorization",
        "summary":"ACCESS DENIED","requirements_state":"declared_not_verified",
        "retry":"no_automatic_retry","remote_effect":"not_inferred",
        "guide":{"operation":"sample.inspect","selectors":{"mode":"echo"},
            "intent":"inspect_sample","keywords":["READ"],"requirements":[],
            "next_actions":[],"documentation":[]}}});
    guidance::enrich(&r, &r.entries[0], &mut response).unwrap();
    assert_eq!(
        response["diagnostic"]["inspect_argv"],
        json!([
            "zix",
            "describe",
            "sample",
            "--operation",
            "sample.inspect",
            "--argument",
            "mode=echo",
            "--format",
            "json"
        ])
    );
}
