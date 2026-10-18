mod support;
use serde_json::json;
use support::*;
#[test]
fn imported_routes_and_reviewed_rules_produce_one_fixed_origin_get() {
    let p = provider();
    let req = request();
    let plan = p.prepare(&req).unwrap();
    assert_eq!(plan.origin, "https://service.example.invalid");
    assert_eq!(
        plan.url,
        format!(
            "https://service.example.invalid/api/accounts/{ACCOUNT}/items/synthetic-item?limit=20"
        )
    );
    assert_eq!(
        p.manifest().credential_environment,
        ["ZIX_SAMPLE_ACCESS_TOKEN"]
    );
    assert!(p.manifest().commands.iter().all(|c| c.effect == "read"));
}
#[test]
fn invalid_scopes_unknown_inputs_and_unreviewed_commands_fail_without_transport() {
    let p = provider();
    for input in [
        "../other",
        "%2fother",
        "other/child",
        "space value",
        "https://other.invalid",
        "",
    ] {
        let mut req = request();
        req.arguments.insert("item".into(), input.into());
        assert!(p.prepare(&req).is_err());
    }
    let mut req = request();
    req.arguments
        .insert("unexpected".into(), "forbidden".into());
    assert!(p.prepare(&req).is_err());
    let mut req = request();
    req.operation = "item.delete".into();
    assert!(p.prepare(&req).is_err());
    let mut req = request();
    req.arguments
        .insert("account".into(), "not-an-account".into());
    assert!(p.prepare(&req).is_err());
}
#[test]
fn protocol_limits_and_ids_are_checked_before_network_or_credentials() {
    let p = provider();
    for timeout in [0, 49, 20001] {
        let mut req = request();
        req.limits.timeout_ms = timeout;
        assert!(p.prepare(&req).is_err());
    }
    for maximum in [0, 1023, 131073] {
        let mut req = request();
        req.limits.maximum_response_bytes = maximum;
        assert!(p.prepare(&req).is_err());
    }
    let mut req = request();
    req.id = "not-a-request-id".into();
    assert!(p.prepare(&req).is_err());
}
#[test]
fn policy_pins_methods_origins_arguments_and_known_schema() {
    let (mut routes, policy) = documents();
    routes["operations"][0]["method"] = json!("DELETE");
    assert!(load(&routes, &policy).is_err());
    let mut policy = policy;
    policy["routes_sha256"] = json!(zix_cli::files::digest(
        &serde_json::to_vec(&routes).unwrap()
    ));
    assert!(load(&routes, &policy).is_err());
    for origin in [
        "http://service.example.invalid",
        "https://u@service.example.invalid",
        "https://service.example.invalid/path",
    ] {
        let (r, mut p) = documents();
        p["origin"] = json!(origin);
        assert!(load(&r, &p).is_err());
    }
    let (r, mut p) = documents();
    p["operations"][0]["command"]["effect"] = json!("write");
    assert!(load(&r, &p).is_err());
    let (r, mut p) = documents();
    p["operations"][0]["identity"][0]["expected"] = json!("{unbound}");
    assert!(load(&r, &p).is_err());
}
