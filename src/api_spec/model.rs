use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Openapi,
    Discovery,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub namespace: String,
    pub command_id: String,
    pub manifest_sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub id: String,
    pub binding: Option<Binding>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub schema: String,
    pub source_sha256: String,
    pub format: Format,
    pub operations: Vec<Selection>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub format: Format,
    pub specification_version: String,
    pub sha256: String,
    pub api: String,
    pub version: String,
    pub revision: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: String,
    pub method: String,
    pub path: String,
    pub contract: Value,
    pub context: Value,
    pub references: BTreeMap<String, Value>,
    pub binding: Option<Binding>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema: String,
    pub source: Source,
    pub operations: Vec<Operation>,
}
