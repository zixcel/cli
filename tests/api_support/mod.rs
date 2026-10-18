use serde_json::{Value, json};
use zix_cli::{
    api_spec::{self, Catalog},
    files,
};
pub fn source() -> Value {
    serde_json::from_str(include_str!(
        "../../playground/api-catalog/service.openapi.json"
    ))
    .unwrap()
}
pub fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
pub fn review(source: &[u8], format: &str, id: &str) -> Value {
    json!({"schema":"zix-api-review/v1","source_sha256":files::digest(source),
        "format":format,"operations":[{"id":id,"binding":null}]})
}
pub fn compile(value: &Value) -> Catalog {
    let source = bytes(value);
    api_spec::import(
        &source,
        &bytes(&review(&source, "openapi", "sample.inspect")),
    )
    .unwrap()
}
