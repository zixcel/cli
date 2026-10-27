use super::Trigger;
use serde_json::Value;
pub(super) fn classify(v: &Value) -> Trigger {
    let error = v["error"].as_str().unwrap_or("");
    let result = &v["result"];
    if result["state"] == "outcome_unknown"
        || error.contains("outcome_unknown")
        || error.contains("unresolved")
    {
        return Trigger::OutcomeUnknown;
    }
    if v["http_status"] == 403
        || error.ends_with("permission_denied")
        || error.contains("access_denied")
        || error == "authorization_denied"
        || reason(result, "permission_denied", 0)
    {
        return Trigger::Authorization;
    }
    if v["http_status"] == 401
        || error.contains("authentication")
        || error.contains("audience")
        || error.contains("expired_token")
        || error == "invalid_grant"
        || reason(result, "authentication_rejected", 0)
    {
        return Trigger::Authentication;
    }
    if error.contains("missing")
        || error.contains("invalid_argument")
        || error.contains("invalid_configuration_selector")
    {
        return Trigger::Context;
    }
    if !error.is_empty() {
        if v["http_status"] == 429 || error.contains("rate_limited") {
            return Trigger::RateLimited;
        }
        return Trigger::Other;
    }
    if incomplete(result, 0) {
        return Trigger::Incomplete;
    }
    Trigger::Observed
}
fn incomplete(v: &Value, depth: usize) -> bool {
    if depth > 8 {
        return true;
    }
    match v {
        Value::Object(o) => {
            if o.get("evaluated") == Some(&Value::Bool(false)) {
                return false;
            }
            o.get("complete") == Some(&Value::Bool(false))
                || o.get("status").and_then(Value::as_str).is_some_and(|s| {
                    ["unobserved", "partial", "unsupported", "denied"].contains(&s)
                })
                || o.get("state")
                    .and_then(Value::as_str)
                    .is_some_and(|s| ["unobserved", "partial", "denied"].contains(&s))
                || o.values().any(|v| incomplete(v, depth + 1))
        }
        Value::Array(a) => a.iter().any(|v| incomplete(v, depth + 1)),
        _ => false,
    }
}
fn reason(v: &Value, wanted: &str, depth: usize) -> bool {
    if depth > 8 {
        return false;
    }
    match v {
        Value::Object(o) => {
            o.get("reason").and_then(Value::as_str) == Some(wanted)
                || o.values().any(|v| reason(v, wanted, depth + 1))
        }
        Value::Array(a) => a.iter().any(|v| reason(v, wanted, depth + 1)),
        _ => false,
    }
}
