use super::{Operation, Selection, references};
use crate::{Error, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub fn select(
    root: &Value,
    selection: &Selection,
    format: &super::Format,
    documents: &super::documents::Documents<'_>,
) -> Result<Operation> {
    let paths = root["paths"]
        .as_object()
        .ok_or(Error("invalid_openapi_paths"))?;
    let mut found = BTreeMap::new();
    for (path, item) in paths {
        let item = item.as_object().ok_or(Error("invalid_openapi_path"))?;
        if item.contains_key("$ref") {
            return Err(Error("unsupported_path_reference"));
        }
        for method in [
            "get", "put", "post", "delete", "options", "head", "patch", "trace",
        ] {
            let Some(operation) = item.get(method) else {
                continue;
            };
            let Some(id) = operation.get("operationId").and_then(Value::as_str) else {
                continue;
            };
            if id == selection.id && found.insert(id, (path, method, item, operation)).is_some() {
                return Err(Error("duplicate_api_operation"));
            }
        }
    }
    let (path, method, item, contract) = found
        .get(selection.id.as_str())
        .ok_or(Error("unknown_api_operation"))?;
    let context = json!({"info":root.get("info"),"jsonSchemaDialect":root.get("jsonSchemaDialect"),"servers":root.get("servers"),"security":root.get("security"),
        "securitySchemes":root.pointer("/components/securitySchemes"),
        "pathParameters":item.get("parameters"),"pathServers":item.get("servers"),
        "host":root.get("host"),"basePath":root.get("basePath"),"schemes":root.get("schemes"),
        "consumes":root.get("consumes"),"produces":root.get("produces"),
        "securityDefinitions":root.get("securityDefinitions")});
    let references = references::closure(&[contract, &context], format, documents)?;
    Ok(Operation {
        id: selection.id.clone(),
        method: method.to_ascii_uppercase(),
        path: (*path).clone(),
        contract: (*contract).clone(),
        context,
        references,
        binding: selection.binding.clone(),
    })
}
