mod api_support;
use api_support::*;
use serde_json::json;
use zix_cli::api_spec;
#[test]
fn schema_constraints_scopes_servers_and_binding_drift_require_review() {
    let before = compile(&source());
    let mut changes = Vec::new();
    let mut value = source();
    value["components"]["schemas"]["Inspection"]["required"] = json!(["value"]);
    changes.push(value);
    let mut value = source();
    value["paths"]["/inspection"]["get"]["security"] = json!([{"oauth":["new.scope"]}]);
    changes.push(value);
    let mut value = source();
    value["servers"][0]["url"] = json!("https://different.example.invalid");
    changes.push(value);
    for value in changes {
        let after = compile(&value);
        let report = api_spec::diff(&before, &after);
        assert_eq!(report["changed"], json!(["sample.inspect"]));
        assert_eq!(report["source_changed"], true);
        assert_eq!(report["automatic_adoption"], false);
    }
    assert_eq!(api_spec::diff(&before, &before)["equal"], true);
    let mut after = before.clone();
    after.operations[0].binding = Some(api_spec::Binding {
        namespace: "sample".into(),
        command_id: "sample.inspect".into(),
        manifest_sha256: "0".repeat(64),
    });
    assert_eq!(
        api_spec::diff(&before, &after)["changed"],
        json!(["sample.inspect"])
    );
}
#[test]
fn added_upstream_operations_are_not_enabled_and_removals_fail_import() {
    let before = compile(&source());
    let mut value = source();
    value["paths"]["/unreviewed"] =
        json!({"delete":{"operationId":"sample.delete","responses":{}}});
    let after = compile(&value);
    assert_eq!(after.operations.len(), 1);
    let report = api_spec::diff(&before, &after);
    assert_eq!(report["source_changed"], true);
    assert_eq!(report["added"], json!([]));
    value["paths"]
        .as_object_mut()
        .unwrap()
        .remove("/inspection");
    let input = bytes(&value);
    assert!(
        api_spec::import(&input, &bytes(&review(&input, "openapi", "sample.inspect"))).is_err()
    );
}
#[test]
fn references_are_closed_locally_and_missing_external_or_excessive_refs_fail() {
    for reference in [
        "https://example.invalid/schema",
        "#/components/schemas/Missing",
        "other.json#/Thing",
    ] {
        let mut value = source();
        value["components"]["schemas"]["Inspection"] = json!({"$ref":reference});
        let input = bytes(&value);
        assert!(
            api_spec::import(&input, &bytes(&review(&input, "openapi", "sample.inspect"))).is_err()
        );
    }
    let mut value = source();
    for i in 0..130 {
        let name = if i == 0 {
            "Inspection".into()
        } else {
            format!("S{i}")
        };
        value["components"]["schemas"][name] =
            json!({"$ref":format!("#/components/schemas/S{}",i+1)});
    }
    let input = bytes(&value);
    assert!(
        api_spec::import(&input, &bytes(&review(&input, "openapi", "sample.inspect"))).is_err()
    );
}
#[test]
fn duplicate_operation_ids_and_path_references_are_not_ambiguous() {
    let mut value = source();
    value["paths"]["/other"] = value["paths"]["/inspection"].clone();
    let input = bytes(&value);
    assert!(
        api_spec::import(&input, &bytes(&review(&input, "openapi", "sample.inspect"))).is_err()
    );
    value = source();
    value["paths"]["/other"] = json!({"$ref":"#/paths/~1inspection"});
    let input = bytes(&value);
    assert!(
        api_spec::import(&input, &bytes(&review(&input, "openapi", "sample.inspect"))).is_err()
    );
}
