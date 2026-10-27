use crate::{Error, Result};
use serde_json::Value;
#[cfg(test)]
pub(crate) fn response(
    bytes: &[u8],
    id: &str,
    operation: &str,
    secrets: &[String],
) -> Result<Value> {
    response_with_protocol(bytes, id, operation, secrets, "zix-provider/v1")
}
pub(crate) fn response_with_protocol(
    bytes: &[u8],
    id: &str,
    operation: &str,
    secrets: &[String],
    protocol: &str,
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
    if object.keys().any(|k| {
        ![
            "schema",
            "id",
            "operation",
            "result",
            "error",
            "http_status",
            "diagnostic",
        ]
        .contains(&k.as_str())
    }) || value["schema"] != protocol
        || value["id"] != id
        || value["operation"] != operation
        || object.contains_key("result") == object.contains_key("error")
    {
        return Err(Error("protocol_rejected"));
    }
    if protocol == "zix-provider/v2" {
        let diagnostic: zix_operation_contracts::guidance::Diagnostic =
            serde_json::from_value(value["diagnostic"].clone())
                .map_err(|_| Error("invalid_diagnostic"))?;
        if !diagnostic.validate()
            || diagnostic.guide.operation != operation
            || (object.contains_key("error")
                && diagnostic.state == zix_operation_contracts::guidance::Resolution::Observed)
            || (value.pointer("/result/state").and_then(Value::as_str) == Some("outcome_unknown")
                && diagnostic.reason != zix_operation_contracts::guidance::Trigger::OutcomeUnknown)
        {
            return Err(Error("invalid_diagnostic"));
        }
    } else if object.contains_key("diagnostic") {
        return Err(Error("protocol_rejected"));
    }
    if let Some(status) = object.get("http_status")
        && (!object.contains_key("error")
            || !status.as_u64().is_some_and(|n| (100..=599).contains(&n)))
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
pub fn contains_secret(value: &Value, secrets: &[String]) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_numeric_http_status_is_allowed_only_with_an_error() {
        let base = serde_json::json!({"schema":"zix-provider/v1","id":"sample-id","operation":"sample.read","error":"http_failed","http_status":400});
        let check = |value: &Value| {
            response(
                &serde_json::to_vec(value).unwrap(),
                "sample-id",
                "sample.read",
                &[],
            )
        };
        assert_eq!(check(&base).unwrap()["http_status"], 400);
        for invalid in [
            serde_json::json!(99),
            serde_json::json!(600),
            serde_json::json!(400.5),
            serde_json::json!("400"),
            Value::Null,
        ] {
            let mut value = base.clone();
            value["http_status"] = invalid;
            assert!(check(&value).is_err());
        }
        let mut value = base.clone();
        value.as_object_mut().unwrap().remove("error");
        value["result"] = serde_json::json!({});
        assert!(check(&value).is_err());
        let mut value = base.clone();
        value["provider_message"] = serde_json::json!("SYNTHETIC_PRIVATE");
        assert!(check(&value).is_err());
    }
}
