//! Descriptive meaning and prerequisites; never execution authority or an inference engine.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod classification;
mod observation;
mod validation;
pub use validation::valid;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema: String,
    pub namespace: String,
    pub package: String,
    pub version: String,
    pub entries: Vec<Guide>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Guide {
    pub operation: String,
    #[serde(default)]
    pub selectors: BTreeMap<String, String>,
    pub intent: String,
    pub keywords: Vec<String>,
    pub requirements: Vec<Requirement>,
    pub next_actions: Vec<Action>,
    pub documentation: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub kind: RequirementKind,
    pub value: String,
    #[serde(default = "operation_subject")]
    pub subject: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<String>,
}
fn operation_subject() -> String {
    "operation".into()
}
impl Requirement {
    pub fn new(kind: RequirementKind, value: String) -> Self {
        Self {
            kind,
            value,
            subject: operation_subject(),
            alternatives: vec![],
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequirementKind {
    Audience,
    OauthScope,
    Consent,
    Role,
    TargetScope,
    Context,
    Runtime,
    ProviderPermission,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub namespace: String,
    pub operation: String,
    pub purpose: String,
    #[serde(default)]
    pub arguments: BTreeMap<String, String>,
    pub on: Vec<Trigger>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Observed,
    Context,
    Authentication,
    Authorization,
    Incomplete,
    OutcomeUnknown,
    RateLimited,
    Other,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub schema: String,
    pub state: Resolution,
    pub reason: Trigger,
    pub summary: String,
    pub guide: Guide,
    pub requirements_state: String,
    pub retry: String,
    pub remote_effect: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Resolution {
    Observed,
    NeedsResolution,
}
impl Catalog {
    pub fn select(&self, operation: &str, arguments: &BTreeMap<String, String>) -> Option<&Guide> {
        self.entries
            .iter()
            .filter(|g| {
                g.operation == operation
                    && g.selectors.iter().all(|(k, v)| arguments.get(k) == Some(v))
            })
            .max_by_key(|g| g.selectors.len())
    }
}
