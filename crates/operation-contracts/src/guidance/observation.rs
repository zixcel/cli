use super::classification::classify;
use super::{Diagnostic, Guide, Resolution, Trigger};
use serde_json::Value;
impl Diagnostic {
    pub fn observe(guide: Guide, response: &Value) -> Self {
        let reason = classify(response);
        let (label, next) = match reason {
            Trigger::Observed => ("OBSERVED", "review_next_action"),
            Trigger::Context => ("CONTEXT MISSING", "supply_required_context"),
            Trigger::Authentication => ("AUTH REJECTED", "check_audience_expiry_identity"),
            Trigger::Authorization => ("ACCESS DENIED", "check_scope_consent_role_target"),
            Trigger::Incomplete => ("EVIDENCE INCOMPLETE", "inspect_missing_observation"),
            Trigger::OutcomeUnknown => ("OUTCOME UNKNOWN", "readback_before_new_plan"),
            Trigger::RateLimited => ("RATE LIMITED", "wait_before_authorized_retry"),
            Trigger::Other => ("OPERATION FAILED", "inspect_requirements"),
        };
        let prefix = guide
            .keywords
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        Diagnostic {
            schema: "zix-diagnostic/v1".into(),
            state: if reason == Trigger::Observed {
                Resolution::Observed
            } else {
                Resolution::NeedsResolution
            },
            reason,
            summary: format!("{prefix} {label}; NEXT {next}."),
            guide,
            requirements_state: "declared_not_verified".into(),
            retry: "no_automatic_retry".into(),
            remote_effect: if reason == Trigger::OutcomeUnknown {
                "unknown"
            } else {
                "not_inferred"
            }
            .into(),
        }
    }
}
