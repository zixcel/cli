mod api_support;
use api_support::*;
use serde_json::json;
use zix_cli::api_spec;
#[test]
fn openapi_import_selects_referenced_contracts_without_granting_authority() {
    let catalog = compile(&source());
    assert_eq!(catalog.operations.len(), 1);
    assert_eq!(catalog.source.version, "1");
    assert_eq!(catalog.source.specification_version, "3.1.0");
    assert!(catalog.source.revision.is_none());
    let op = &catalog.operations[0];
    assert_eq!(op.method, "GET");
    assert_eq!(op.path, "/inspection");
    assert!(op.binding.is_none());
    assert_eq!(
        op.references["/components/schemas/Inspection"]["type"],
        "object"
    );
    assert_eq!(
        api_spec::parse(&bytes(&serde_json::to_value(&catalog).unwrap())).unwrap(),
        catalog
    );
}
#[test]
fn discovery_handles_nested_resources_global_parameters_and_recursive_schemas() {
    let source = bytes(
        &json!({"discoveryVersion":"v1","name":"sample","version":"v1","revision":"1",
        "rootUrl":"https://service.example.invalid/","servicePath":"v1/",
        "parameters":{"quotaUser":{"type":"string"}},"resources":{"items":{"methods":{
            "get":{"id":"sample.inspect","httpMethod":"POST","path":"items/{name}",
                "scopes":["synthetic.read"],"response":{"$ref":"Item"}}}}},
        "schemas":{"Item":{"type":"object","properties":{"next":{"$ref":"Item"}}}}}),
    );
    let catalog = api_spec::import(
        &source,
        &bytes(&review(&source, "discovery", "sample.inspect")),
    )
    .unwrap();
    assert_eq!(catalog.source.revision.as_deref(), Some("1"));
    assert_eq!(catalog.operations[0].references.len(), 1);
    assert_eq!(catalog.operations[0].method, "POST");
    assert!(catalog.operations[0].binding.is_none());
    assert_eq!(
        catalog.operations[0].context["parameters"]["quotaUser"]["type"],
        "string"
    );
}
#[test]
fn source_hash_unknown_methods_duplicate_selections_and_future_formats_fail_closed() {
    let source = bytes(&source());
    let mut r = review(&source, "openapi", "sample.inspect");
    r["source_sha256"] = json!("0".repeat(64));
    assert!(api_spec::import(&source, &bytes(&r)).is_err());
    r = review(&source, "openapi", "missing");
    assert!(api_spec::import(&source, &bytes(&r)).is_err());
    r = review(&source, "openapi", "sample.inspect");
    let duplicate = r["operations"][0].clone();
    r["operations"].as_array_mut().unwrap().push(duplicate);
    assert!(api_spec::import(&source, &bytes(&r)).is_err());
    let mut value = api_support::source();
    for version in ["3.2.0", "2.0", "3.1.bad", "3.1.0-extra"] {
        value["openapi"] = json!(version);
        let input = bytes(&value);
        assert!(
            api_spec::import(&input, &bytes(&review(&input, "openapi", "sample.inspect"))).is_err()
        );
    }
}
#[test]
fn duplicate_json_keys_and_oversized_or_deep_inputs_are_rejected() {
    let mut source = bytes(&source());
    source.splice(1..1, br#""openapi":"3.0.0","#.iter().copied());
    assert!(
        api_spec::import(
            &source,
            &bytes(&review(&source, "openapi", "sample.inspect"))
        )
        .is_err()
    );
    let too_big = vec![b' '; api_spec::SPEC_LIMIT + 1];
    assert!(
        api_spec::import(
            &too_big,
            &bytes(&review(&too_big, "openapi", "sample.inspect"))
        )
        .is_err()
    );
    let mut value = api_support::source();
    let mut nested = json!(null);
    for _ in 0..66 {
        nested = json!([nested]);
    }
    value["extra"] = nested;
    let input = bytes(&value);
    assert!(
        api_spec::import(&input, &bytes(&review(&input, "openapi", "sample.inspect"))).is_err()
    );
}
#[test]
fn decimal_and_large_integer_constraints_are_not_rounded_in_catalogs() {
    let raw = include_str!("../playground/api-catalog/service.openapi.json")
        .replace("\"type\": \"string\"", "\"type\": \"number\", \"minimum\": 0.1234567890123456789012345, \"maximum\": 184467440737095516170");
    let catalog = api_spec::import(
        raw.as_bytes(),
        &bytes(&review(raw.as_bytes(), "openapi", "sample.inspect")),
    )
    .unwrap();
    let encoded = serde_json::to_string(&catalog).unwrap();
    assert!(encoded.contains("0.1234567890123456789012345"));
    assert!(encoded.contains("184467440737095516170"));
}
