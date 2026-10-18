mod api_support;
use api_support::*;
use serde_json::{Value, json};
use zix_cli::{api_spec, files};
fn inputs() -> (Vec<u8>, Value, Value) {
    let mut source = source();
    source["components"]["schemas"]["Inspection"] =
        json!({"$ref":"../common/item.json#/definitions/Item"});
    let source = bytes(&source);
    let mut review = review(&source, "openapi", "sample.inspect");
    review["source_uri"] = json!("https://schemas.example.invalid/service/root.json");
    let uri = "https://schemas.example.invalid/common/item.json";
    let document=serde_json::to_string(&json!({"definitions":{"Item":{"type":"object","properties":{"next":{"$ref":"#/definitions/Item"}}}}})).unwrap();
    review["reference_sources"] = json!({uri:files::digest(document.as_bytes())});
    (source, review, json!({"documents":{uri:document}}))
}
#[test]
fn reviewed_documents_resolve_relative_and_recursive_refs_without_network() {
    let (s, r, b) = inputs();
    let c = api_spec::project_with_documents(&s, &bytes(&r), &bytes(&b)).unwrap();
    let baseline = compile(&source());
    assert_ne!(
        baseline.operations[0].references,
        c.operations[0].references
    );
    let key = "https://schemas.example.invalid/common/item.json#/definitions/Item";
    assert_eq!(c.operations[0].references[key]["type"], "object");
    assert_eq!(c.source.reference_sources.len(), 1);
}
#[test]
fn missing_unreviewed_changed_or_unsafe_documents_are_refused() {
    let (s, r, mut b) = inputs();
    assert!(api_spec::project(&s, &bytes(&r)).is_err());
    b["documents"]["https://schemas.example.invalid/common/item.json"] = json!("{}");
    assert!(api_spec::project_with_documents(&s, &bytes(&r), &bytes(&b)).is_err());
    let (s, mut r, b) = inputs();
    for uri in [
        "file:///private/source.json",
        "https://user:password@schemas.example.invalid/root.json",
        "https://schemas.example.invalid/root.json?access_token=synthetic",
    ] {
        r["source_uri"] = json!(uri);
        assert!(api_spec::project_with_documents(&s, &bytes(&r), &bytes(&b)).is_err());
    }
    let (s, r, mut b) = inputs();
    b["documents"]["https://unreviewed.example.invalid/x.json"] = json!("{}");
    assert!(api_spec::project_with_documents(&s, &bytes(&r), &bytes(&b)).is_err());
}
#[test]
fn reference_schema_updates_change_both_evidence_and_selected_contract() {
    let (s, mut r, mut b) = inputs();
    let before = api_spec::project_with_documents(&s, &bytes(&r), &bytes(&b)).unwrap();
    let uri = "https://schemas.example.invalid/common/item.json";
    let raw = "{\"definitions\":{\"Item\":{\"type\":\"string\"}}}";
    b["documents"][uri] = json!(raw);
    r["reference_sources"][uri] = json!(files::digest(raw.as_bytes()));
    let after = api_spec::project_with_documents(&s, &bytes(&r), &bytes(&b)).unwrap();
    let diff = api_spec::diff(&before, &after);
    assert_eq!(diff["source_changed"], true);
    assert_eq!(diff["changed"], json!(["sample.inspect"]));
}
