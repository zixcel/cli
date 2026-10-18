use super::Format;
use crate::{Error, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub fn closure(
    root: &Value,
    values: &[&Value],
    format: &Format,
) -> Result<BTreeMap<String, Value>> {
    let mut pending = BTreeSet::new();
    for v in values {
        find(v, format, &mut pending)?;
    }
    let mut resolved = BTreeMap::new();
    while let Some(reference) = pending.pop_first() {
        if resolved.contains_key(&reference) {
            continue;
        }
        if resolved.len() >= 128 {
            return Err(Error("api_reference_limit"));
        }
        let value = root
            .pointer(&reference)
            .ok_or(Error("missing_api_reference"))?;
        find(value, format, &mut pending)?;
        resolved.insert(reference, value.clone());
    }
    Ok(resolved)
}
fn find(value: &Value, format: &Format, found: &mut BTreeSet<String>) -> Result<()> {
    match value {
        Value::Object(fields) => {
            if let Some(reference) = fields.get("$ref") {
                let r = reference.as_str().ok_or(Error("invalid_api_reference"))?;
                let pointer = match format {
                    Format::Openapi => r
                        .strip_prefix('#')
                        .filter(|p| p.starts_with('/'))
                        .ok_or(Error("external_api_reference"))?
                        .to_owned(),
                    Format::Discovery
                        if !r.is_empty()
                            && r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') =>
                    {
                        format!("/schemas/{r}")
                    }
                    _ => return Err(Error("external_api_reference")),
                };
                found.insert(pointer);
            }
            for v in fields.values() {
                find(v, format, found)?;
            }
        }
        Value::Array(values) => {
            for v in values {
                find(v, format, found)?;
            }
        }
        _ => {}
    }
    Ok(())
}
