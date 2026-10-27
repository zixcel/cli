use serde_json::{Value, json};
pub fn catalog() -> Value {
    json!({"schema":"zix-guidance-catalog/v1","namespace":"sample","package":"sample-provider","version":"0.1.0","entries":[{
        "operation":"sample.inspect","selectors":{},"intent":"inspect_sample","keywords":["SAMPLE","READ"],
        "requirements":[{"kind":"oauth_scope","value":"sample.read"}],"documentation":[],
        "next_actions":[{"namespace":"sample","operation":"sample.inspect","purpose":"inspect_evidence","arguments":{"mode":"echo"},"on":["observed","context","authentication","authorization","incomplete","outcome_unknown","other"]}]}]})
}
pub fn diagnostic(mode: &str) -> Value {
    let (state, reason, effect) = match mode {
        "v2-denied" => ("NeedsResolution", "authorization", "not_inferred"),
        "v2-unknown" => ("NeedsResolution", "outcome_unknown", "unknown"),
        "v2-incomplete" => ("NeedsResolution", "incomplete", "not_inferred"),
        _ => ("Observed", "observed", "not_inferred"),
    };
    json!({"schema":"zix-diagnostic/v1","state":state,"reason":reason,"summary":"SAMPLE READ; NEXT inspect_evidence.","guide":catalog()["entries"][0],"requirements_state":"declared_not_verified","retry":"no_automatic_retry","remote_effect":effect})
}
