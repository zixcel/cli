mod support;
use serde_json::json;
use zix_native_provider::{Error, guidance};
use zix_operation_contracts::guidance::{Catalog, Resolution, Trigger};
fn catalog() -> Catalog {
    guidance::catalog(support::provider().manifest(),br#"[{"operation":"item.read","selectors":{},"intent":"read_item","keywords":["SERVICE","READ"],"requirements":[{"kind":"oauth_scope","value":"items.read"}],"next_actions":[],"documentation":[]}]"#).unwrap()
}
#[test]
fn forbidden_requires_resolution_without_claiming_missing_scope() {
    let c = catalog();
    let r = support::request();
    let d = guidance::diagnose(
        &c,
        &r,
        &json!({"error":"permission_denied","http_status":403}),
    )
    .unwrap();
    assert_eq!(d.state, Resolution::NeedsResolution);
    assert_eq!(d.reason, Trigger::Authorization);
    assert_eq!(d.requirements_state, "declared_not_verified");
    assert!(d.summary.contains("scope_consent_role_target"));
    assert_eq!(d.retry, "no_automatic_retry");
}
#[test]
fn unknown_mutations_and_nested_partial_observations_never_become_observed() {
    let c = catalog();
    let r = support::request();
    let d = guidance::diagnose(
        &c,
        &r,
        &json!({"result":{"state":"outcome_unknown","failure":{"http_status":401}}}),
    )
    .unwrap();
    assert_eq!(d.reason, Trigger::OutcomeUnknown);
    assert_eq!(d.remote_effect, "unknown");
    for response in [
        json!({"result":{"state":"verification_pending"}}),
        json!({"error":"unknown_result"}),
    ] {
        assert_eq!(
            guidance::diagnose(&c, &r, &response).unwrap().reason,
            Trigger::OutcomeUnknown
        );
    }
    let d = guidance::diagnose(
        &c,
        &r,
        &json!({"result":{"observation":{"complete":false,"data":[]}}}),
    )
    .unwrap();
    assert_eq!(d.reason, Trigger::Incomplete);
    let d = guidance::diagnose(&c, &r, &json!({"result":{"complete":true,"data":[]}})).unwrap();
    assert_eq!(d.state, Resolution::Observed);
    let d=guidance::diagnose(&c,&r,&json!({"result":{"observation":{"complete":true,"routing_guard":{"evaluated":false,"complete":false}}}})).unwrap();
    assert_eq!(d.state, Resolution::Observed);
    for (reason, expected) in [
        ("authentication_rejected", Trigger::Authentication),
        ("permission_denied", Trigger::Authorization),
    ] {
        let d=guidance::diagnose(&c,&r,&json!({"result":{"observations":{"zone":{"state":"denied","reason":reason,"data":null}}}})).unwrap();
        assert_eq!(d.reason, expected);
    }
}
#[test]
fn selector_specific_requirements_take_priority_and_ambiguous_rules_reject() {
    let mut c = catalog();
    let mut g = c.entries[0].clone();
    g.selectors.insert("item".into(), "synthetic-item".into());
    g.intent = "read_selected_item".into();
    c.entries.push(g.clone());
    assert!(c.validate());
    assert_eq!(
        c.select("item.read", &support::request().arguments)
            .unwrap()
            .intent,
        "read_selected_item"
    );
    let mut other = g.clone();
    other.selectors.clear();
    other
        .selectors
        .insert("account".into(), support::ACCOUNT.into());
    c.entries.push(other);
    assert!(!c.validate());
    g.selectors.clear();
    g.selectors.insert("undeclared".into(), "value".into());
    assert_eq!(
        guidance::catalog(
            support::provider().manifest(),
            &serde_json::to_vec(&vec![g]).unwrap()
        )
        .unwrap_err(),
        Error("invalid_guidance_selector")
    );
}
#[test]
fn provider_messages_shell_text_unknown_fields_and_unbounded_metadata_are_rejected() {
    let mut c = serde_json::to_value(catalog()).unwrap();
    c["entries"][0]["provider_message"] = json!("untrusted raw response");
    assert!(serde_json::from_value::<Catalog>(c).is_err());
    let mut c = catalog();
    c.entries[0].intent = "run $(unsafe)".into();
    assert!(!c.validate());
    let mut c = catalog();
    c.entries[0].keywords = vec!["X".repeat(33)];
    assert!(!c.validate());
    let d = guidance::diagnose(
        &catalog(),
        &support::request(),
        &json!({"error":"credential_missing"}),
    )
    .unwrap();
    assert_eq!(d.reason, Trigger::Context);
    let source = serde_json::to_string(&catalog().entries).unwrap();
    let pretty = format!("{}{source}", " ".repeat(68000));
    assert!(guidance::catalog(support::provider().manifest(), pretty.as_bytes()).is_ok());
    let oversized = format!("{}[]", " ".repeat(131072));
    assert!(guidance::catalog(support::provider().manifest(), oversized.as_bytes()).is_err());
}
#[test]
fn every_operation_requires_an_owned_base_definition_and_common_failure_codes_agree() {
    assert_eq!(
        guidance::catalog(support::provider().manifest(), b"[]").unwrap_err(),
        Error("invalid_guidance")
    );
    for (code, reason) in [
        ("auth_required", Trigger::Authentication),
        ("forbidden", Trigger::Authorization),
        ("unknown_outcome", Trigger::OutcomeUnknown),
    ] {
        let d =
            guidance::diagnose(&catalog(), &support::request(), &json!({"error":code})).unwrap();
        assert_eq!(d.reason, reason);
        assert_eq!(d.state, Resolution::NeedsResolution);
    }
}
