use zix_cli::api_spec;
const YAML: &str = "openapi: 3.0.4\ninfo: {title: Synthetic, version: '1'}\nservers: [{url: 'https://example.invalid'}]\npaths:\n  /item:\n    get:\n      operationId: sample.inspect\n      responses:\n        '200': {description: Synthetic}\n";
fn compile(input: &str) -> zix_cli::Result<api_spec::Catalog> {
    let review = serde_json::json!({"schema":"zix-api-review/v1","source_sha256":zix_cli::files::digest(input.as_bytes()),
        "format":"openapi_yaml","operations":[{"id":"sample.inspect","binding":null}]});
    api_spec::import(input.as_bytes(), &serde_json::to_vec(&review).unwrap())
}
#[test]
fn official_style_yaml_is_bounded_and_keeps_raw_source_hash() {
    let catalog = compile(YAML).unwrap();
    assert_eq!(catalog.source.format, api_spec::Format::OpenapiYaml);
    assert_eq!(
        catalog.source.sha256,
        zix_cli::files::digest(YAML.as_bytes())
    );
    assert_eq!(catalog.operations[0].path, "/item");
}
#[test]
fn yaml_anchors_merges_includes_duplicates_and_multiple_documents_fail_closed() {
    for extra in [
        "a: &a {x: 1}\nb: *a\n",
        "a: {<<: {x: 1}}\n",
        "a: !include /private/must-not-open\n",
        "a: 1\na: 2\n",
        "---\na: 1\n",
        "a: !!python/object:thing {}\n",
    ] {
        assert!(compile(&(YAML.to_owned() + extra)).is_err(), "{extra}");
    }
}
#[test]
fn yaml_numeric_limits_are_not_silently_rounded() {
    assert!(compile(&(YAML.to_owned() + "example: 0.1234567890123456789\n")).is_err());
    let c = compile(&(YAML.to_owned() + "example: '0.1234567890123456789'\n")).unwrap();
    assert_eq!(c.operations.len(), 1);
}
