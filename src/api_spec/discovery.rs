use super::{Format, Operation, Selection, references};
use crate::{Error, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub fn select(root: &Value, selection: &Selection) -> Result<Operation> {
    let mut found = BTreeMap::new();
    methods(root, &mut found)?;
    let contract = found
        .get(&selection.id)
        .ok_or(Error("unknown_api_operation"))?;
    let method = contract["httpMethod"]
        .as_str()
        .ok_or(Error("invalid_discovery_method"))?;
    let path = contract["path"]
        .as_str()
        .ok_or(Error("invalid_discovery_path"))?;
    let context = json!({"rootUrl":root.get("rootUrl"),"servicePath":root.get("servicePath"),
        "baseUrl":root.get("baseUrl"),"parameters":root.get("parameters"),"auth":root.get("auth")});
    let references = references::closure(root, &[contract, &context], &Format::Discovery)?;
    Ok(Operation {
        id: selection.id.clone(),
        method: method.into(),
        path: path.into(),
        contract: (*contract).clone(),
        context,
        references,
        binding: selection.binding.clone(),
    })
}
fn methods<'a>(value: &'a Value, found: &mut BTreeMap<String, &'a Value>) -> Result<()> {
    if let Some(methods) = value.get("methods") {
        for m in methods
            .as_object()
            .ok_or(Error("invalid_discovery_methods"))?
            .values()
        {
            let id = m["id"].as_str().ok_or(Error("invalid_discovery_method"))?;
            if found.insert(id.into(), m).is_some() {
                return Err(Error("duplicate_api_operation"));
            }
        }
    }
    if let Some(resources) = value.get("resources") {
        for resource in resources
            .as_object()
            .ok_or(Error("invalid_discovery_resources"))?
            .values()
        {
            methods(resource, found)?;
        }
    }
    Ok(())
}
