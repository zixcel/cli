use super::{Action, Catalog, Diagnostic, Guide, Resolution, Trigger};
pub fn valid(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
}
fn identifier(s: &str) -> bool {
    valid(s, 100)
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}
impl Action {
    pub fn validate(&self) -> bool {
        identifier(&self.namespace)
            && identifier(&self.operation)
            && identifier(&self.purpose)
            && self.arguments.len() <= 8
            && self
                .arguments
                .iter()
                .all(|(k, v)| identifier(k) && valid(v, 256))
            && !self.on.is_empty()
            && self.on.len() <= 8
            && self.requires_context.len() <= 4
            && self
                .requires_context
                .iter()
                .all(|(k, v)| identifier(k) && valid(v, 128))
    }
}
impl Guide {
    pub fn validate(&self) -> bool {
        identifier(&self.operation)
            && identifier(&self.intent)
            && self.selectors.len() <= 4
            && self
                .selectors
                .iter()
                .all(|(k, v)| identifier(k) && valid(v, 128))
            && !self.keywords.is_empty()
            && self.keywords.len() <= 8
            && self.keywords.iter().all(|s| {
                valid(s, 32)
                    && s.bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            })
            && self.requirements.len() <= 16
            && self.requirements.iter().all(|r| {
                valid(&r.value, 160)
                    && identifier(&r.subject)
                    && r.alternatives.len() <= 8
                    && r.alternatives.iter().all(|v| valid(v, 160))
            })
            && self.next_actions.len() <= 8
            && self.next_actions.iter().all(Action::validate)
            && self.documentation.len() <= 4
            && self
                .documentation
                .iter()
                .all(|s| valid(s, 512) && s.starts_with("https://"))
    }
}
impl Catalog {
    pub fn validate(&self) -> bool {
        if self.schema != "zix-guidance-catalog/v1"
            || !identifier(&self.namespace)
            || !identifier(&self.package)
            || !valid(&self.version, 64)
            || self.entries.is_empty()
            || self.entries.len() > 96
            || !self.entries.iter().all(Guide::validate)
        {
            return false;
        }
        for (i, a) in self.entries.iter().enumerate() {
            for b in &self.entries[i + 1..] {
                if a.operation == b.operation
                    && a.selectors.len() == b.selectors.len()
                    && a.selectors
                        .iter()
                        .all(|(k, v)| b.selectors.get(k).is_none_or(|w| v == w))
                {
                    return false;
                }
            }
        }
        serde_json::to_vec(self).is_ok_and(|v| v.len() <= 65536)
    }
}
impl Diagnostic {
    pub fn validate(&self) -> bool {
        self.schema == "zix-diagnostic/v1"
            && self.guide.validate()
            && valid(&self.summary, 160)
            && self.requirements_state == "declared_not_verified"
            && self.retry == "no_automatic_retry"
            && ["not_inferred", "unknown"].contains(&self.remote_effect.as_str())
            && ((self.reason == Trigger::Observed && self.state == Resolution::Observed)
                || (self.reason != Trigger::Observed && self.state == Resolution::NeedsResolution))
            && (self.reason != Trigger::OutcomeUnknown || self.remote_effect == "unknown")
    }
}
