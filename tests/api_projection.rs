mod api_support;
use api_support::*;
use serde_json::json;
use zix_cli::api_spec;
#[test]
fn developer_projection_does_not_raise_normal_import_or_runtime_limits() {
    let mut large = bytes(&source());
    large.resize(api_spec::SPEC_LIMIT + 1, b' ');
    let r = bytes(&review(&large, "openapi", "sample.inspect"));
    assert!(api_spec::import(&large, &r).is_err());
    let catalog = api_spec::project(&large, &r).unwrap();
    assert!(api_spec::parse(&serde_json::to_vec(&catalog).unwrap()).is_ok());
}
#[test]
fn swagger2_metadata_and_document_only_example_refs_are_preserved() {
    let value = json!({"swagger":"2.0","info":{"title":"Synthetic","version":"v1"},"host":"example.invalid",
        "schemes":["https"],"basePath":"/api","paths":{"/item":{"get":{"operationId":"sample.inspect",
        "responses":{"200":{"schema":{"$ref":"#/definitions/Item"}}},
        "x-ms-examples":{"synthetic":{"$ref":"./never-read-example.json"}}}}},"definitions":{"Item":{"type":"object"}}});
    let input = bytes(&value);
    let c = api_spec::import(&input, &bytes(&review(&input, "swagger", "sample.inspect"))).unwrap();
    assert_eq!(c.source.specification_version, "2.0");
    assert_eq!(c.operations[0].references.len(), 1);
    assert_eq!(
        c.operations[0].contract["x-ms-examples"]["synthetic"]["$ref"],
        "./never-read-example.json"
    );
    assert_eq!(
        api_spec::routes(&c).operations[0].servers,
        ["https://example.invalid/api"]
    );
}
#[test]
fn unrelated_duplicate_ids_do_not_expand_selected_commands_and_routes_omit_vendor_prose() {
    let mut value = source();
    value["paths"]["/a"] =
        json!({"get":{"operationId":"unselected","description":"DO NOT EXECUTE PROSE"}});
    value["paths"]["/b"] = value["paths"]["/a"].clone();
    let catalog = compile(&value);
    let routes = api_spec::routes(&catalog);
    assert_eq!(routes.operations.len(), 1);
    assert!(
        !serde_json::to_string(&routes)
            .unwrap()
            .contains("DO NOT EXECUTE PROSE")
    );
    assert_eq!(
        routes.operations[0].servers,
        ["https://service.example.invalid"]
    );
}

#[test]
fn overlapping_schema_references_stop_at_serialized_budget_before_expansion() {
    let mut value = source();
    let mut nested = json!({"payload":"x".repeat(1024*1024)});
    for depth in (0..12).rev() {
        let pointer = format!(
            "#/components/schemas/Inspection{}",
            "/nested".repeat(depth + 1)
        );
        nested = json!({"nested":nested,"next":{"$ref":pointer}});
    }
    value["components"]["schemas"]["Inspection"] = nested;
    let input = bytes(&value);
    assert!(
        api_spec::import(&input, &bytes(&review(&input, "openapi", "sample.inspect"))).is_err()
    );
}
