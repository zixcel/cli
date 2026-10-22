use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zix_cli::manifest::Command;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub namespace: String,
    pub package: String,
    pub version: String,
    pub credential_environment: String,
    pub origin: String,
    pub base_path: String,
    pub routes_sha256: String,
    pub operations: Vec<Operation>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub api: String,
    pub command: Command,
    pub path_arguments: BTreeMap<String, String>,
    pub argument_rules: BTreeMap<String, Rule>,
    pub query: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    pub identity: Vec<Identity>,
    pub projection: Projection,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    Identifier,
    Uuid,
    Hex32,
    DecimalIdentifier,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub pointer: String,
    pub expected: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub pointer: String,
    pub collection: bool,
    pub fields: BTreeMap<String, String>,
    pub required: Vec<String>,
    pub success_pointer: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<zix_operation_contracts::Binding>,
    pub schema: String,
    pub id: String,
    pub operation: String,
    pub arguments: BTreeMap<String, String>,
    pub limits: Limits,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub timeout_ms: u64,
    pub maximum_response_bytes: usize,
}
#[derive(Clone, Debug)]
pub struct Prepared {
    pub origin: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
}
