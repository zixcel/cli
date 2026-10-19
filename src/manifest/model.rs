use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub package: String,
    pub version: String,
    pub namespace: String,
    pub platform: String,
    pub protocol: String,
    pub credential_environment: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_environment: Vec<String>,
    pub commands: Vec<Command>,
    pub timeout_ms: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: String,
    pub words: Vec<String>,
    pub effect: String,
    pub description: String,
    pub required_permissions: Vec<String>,
    pub arguments: Vec<Argument>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Argument {
    pub name: String,
    pub required: bool,
    pub maximum: usize,
}
