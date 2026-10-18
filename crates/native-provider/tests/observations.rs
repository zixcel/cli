mod support;
use serde_json::json;
use support::*;
#[test]
fn response_projection_preserves_identity_and_omits_undeclared_fields() {
    let p = provider();
    let req = request();
    let raw = json!({"account":ACCOUNT,"id":"synthetic-item","name":"Synthetic item","private":"not-selected"});
    let result = p.project(&req, &serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(
        result["data"],
        json!({"id":"synthetic-item","name":"Synthetic item"})
    );
    assert_eq!(result["permission_state"], "not_inferred");
    assert_eq!(result["request_cost"], "not_assessed");
    assert!(!result.to_string().contains("not-selected"));
}
#[test]
fn cross_account_response_identity_duplicate_keys_or_wrong_types_are_rejected() {
    let p = provider();
    let req = request();
    for value in [
        json!({"account":"other","id":"synthetic-item"}),
        json!({"account":ACCOUNT,"id":"other"}),
        json!({"account":ACCOUNT,"id":"synthetic-item","name":{"unexpected":"object"}}),
    ] {
        assert!(
            p.project(&req, &serde_json::to_vec(&value).unwrap())
                .is_err()
        );
    }
    let raw = format!("{{\"account\":\"{ACCOUNT}\",\"id\":\"synthetic-item\",\"id\":\"other\"}}");
    assert!(p.project(&req, raw.as_bytes()).is_err());
}
#[test]
fn logical_provider_failure_and_response_budget_are_not_successful_snapshots() {
    let (r, mut policy) = documents();
    policy["operations"][0]["projection"]["success_pointer"] = json!("/success");
    let p = load(&r, &policy).unwrap();
    let req = request();
    let raw = json!({"account":ACCOUNT,"id":"synthetic-item","success":false});
    assert!(p.project(&req, &serde_json::to_vec(&raw).unwrap()).is_err());
    let mut req = request();
    req.limits.maximum_response_bytes = 1024;
    let raw =
        json!({"account":ACCOUNT,"id":"synthetic-item","success":true,"name":"x".repeat(1000)});
    assert!(p.project(&req, &serde_json::to_vec(&raw).unwrap()).is_err());
}
