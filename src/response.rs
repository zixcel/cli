use crate::{Error, Result};
use serde_json::Value;
pub(crate) fn response(
    bytes: &[u8],
    id: &str,
    operation: &str,
    secrets: &[String],
) -> Result<Value> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error("protocol_rejected"))?;
    if secrets.iter().any(|s| text.contains(s)) {
        return Err(Error("provider_secret_output"));
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| Error("protocol_rejected"))?;
    if contains_secret(&value, secrets) {
        return Err(Error("provider_secret_output"));
    }
    let object = value.as_object().ok_or(Error("protocol_rejected"))?;
    if object
        .keys()
        .any(|k| !["schema", "id", "operation", "result", "error"].contains(&k.as_str()))
        || value["schema"] != "zix-provider/v1"
        || value["id"] != id
        || value["operation"] != operation
        || object.contains_key("result") == object.contains_key("error")
    {
        return Err(Error("protocol_rejected"));
    }
    if let Some(error) = object.get("error") {
        let code = error.as_str().ok_or(Error("protocol_rejected"))?;
        if code.is_empty()
            || code.len() > 64
            || !code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        {
            return Err(Error("protocol_rejected"));
        }
    }
    Ok(value)
}
fn contains_secret(value: &Value, secrets: &[String]) -> bool {
    let includes = |text: &str| secrets.iter().any(|secret| text.contains(secret));
    match value {
        Value::String(s) => includes(s),
        Value::Array(a) => a.iter().any(|v| contains_secret(v, secrets)),
        Value::Object(o) => o
            .iter()
            .any(|(k, v)| includes(k) || contains_secret(v, secrets)),
        _ => false,
    }
}
