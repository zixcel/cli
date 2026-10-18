use super::{Format, documents::Documents};
use crate::{Error, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Reference {
    document: String,
    pointer: String,
}
impl Reference {
    fn key(&self, documents: &Documents<'_>) -> String {
        if self.document == documents.source_uri {
            self.pointer.clone()
        } else {
            format!("{}#{}", self.document, self.pointer)
        }
    }
}
pub fn closure(
    values: &[&Value],
    format: &Format,
    documents: &Documents<'_>,
) -> Result<BTreeMap<String, Value>> {
    let mut pending = BTreeSet::new();
    for v in values {
        find(v, format, &documents.source_uri, &mut pending)?;
    }
    let mut resolved = BTreeMap::new();
    let mut remaining = super::SPEC_LIMIT;
    while let Some(reference) = pending.pop_first() {
        let key = reference.key(documents);
        if resolved.contains_key(&key) {
            continue;
        }
        if resolved.len() >= documents.reference_limit {
            return Err(Error("api_reference_limit"));
        }
        let value = documents.value(&reference.document, &reference.pointer)?;
        super::budget::consume(value, &mut remaining)?;
        find(value, format, &reference.document, &mut pending)?;
        resolved.insert(key, value.clone());
    }
    Ok(resolved)
}
fn resolve(r: &str, format: &Format, base: &str) -> Result<Reference> {
    match format {
        Format::Discovery
            if !r.is_empty() && r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') =>
        {
            Ok(Reference {
                document: base.into(),
                pointer: format!("/schemas/{r}"),
            })
        }
        Format::Openapi | Format::OpenapiYaml | Format::Swagger => {
            if let Some(pointer) = r.strip_prefix('#') {
                if !pointer.starts_with('/') || pointer.contains('%') {
                    return Err(Error("invalid_api_reference"));
                }
                return Ok(Reference {
                    document: base.into(),
                    pointer: pointer.into(),
                });
            }
            let mut url = super::documents::uri(base)
                .map_err(|_| Error("external_api_reference"))?
                .join(r)
                .map_err(|_| Error("invalid_api_reference"))?;
            let pointer = url.fragment().unwrap_or("").to_owned();
            if (!pointer.is_empty() && !pointer.starts_with('/')) || pointer.contains('%') {
                return Err(Error("invalid_api_reference"));
            }
            url.set_fragment(None);
            super::documents::uri(url.as_str())?;
            Ok(Reference {
                document: url.into(),
                pointer,
            })
        }
        _ => Err(Error("invalid_api_reference")),
    }
}
fn find(value: &Value, format: &Format, base: &str, found: &mut BTreeSet<Reference>) -> Result<()> {
    match value {
        Value::Object(fields) => {
            if let Some(reference) = fields.get("$ref") {
                found.insert(resolve(
                    reference.as_str().ok_or(Error("invalid_api_reference"))?,
                    format,
                    base,
                )?);
            }
            for (key, v) in fields {
                // Documentation-only vendor examples are preserved, never fetched.
                if key != "x-ms-examples" {
                    find(v, format, base, found)?;
                }
            }
        }
        Value::Array(values) => {
            for v in values {
                find(v, format, base, found)?;
            }
        }
        _ => {}
    }
    Ok(())
}
