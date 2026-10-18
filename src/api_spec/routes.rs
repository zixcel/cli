use super::{Catalog, Source};
use serde::{Deserialize, Serialize};
/// Minimal interface facts; no upstream prose, examples or schema bodies.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub id: String,
    pub method: String,
    pub path: String,
    pub servers: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Routes {
    pub schema: String,
    pub source: Source,
    pub operations: Vec<Route>,
}
pub fn routes(catalog: &Catalog) -> Routes {
    Routes {
        schema: "zix-api-routes/v1".into(),
        source: catalog.source.clone(),
        operations: catalog
            .operations
            .iter()
            .map(|o| Route {
                id: o.id.clone(),
                method: o.method.clone(),
                path: o.path.clone(),
                servers: servers(o),
            })
            .collect(),
    }
}

fn servers(operation: &super::Operation) -> Vec<String> {
    for value in [
        operation.contract.get("servers"),
        operation.context.get("pathServers"),
        operation.context.get("servers"),
    ] {
        if let Some(servers) = value.and_then(serde_json::Value::as_array) {
            return servers
                .iter()
                .filter_map(|s| {
                    s.get("url")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .collect();
        }
    }
    let c = &operation.context;
    if let Some(host) = c.get("host").and_then(serde_json::Value::as_str)
        && operation
            .contract
            .get("schemes")
            .or_else(|| c.get("schemes"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|a| a.iter().any(|s| s == "https"))
    {
        return vec![format!(
            "https://{host}{}",
            c.get("basePath")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
        )];
    }
    if let Some(root) = c.get("rootUrl").and_then(serde_json::Value::as_str) {
        return vec![format!(
            "{root}{}",
            c.get("servicePath")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
        )];
    }
    Vec::new()
}
