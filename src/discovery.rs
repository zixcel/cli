use crate::{Result, registry::Registry};
use serde_json::{Value, json};
pub fn capabilities(registry: &Registry) -> Result<Value> {
    let mut providers = Vec::new();
    for entry in &registry.entries {
        let manifest = entry.manifest()?;
        if entry.enabled {
            providers.push(manifest);
        }
    }
    providers.sort_by(|a, b| a.namespace.cmp(&b.namespace));
    Ok(json!({"schema":"zix-capabilities/v1","providers":providers,"authority":"none"}))
}
pub fn compact_capabilities(registry: &Registry) -> Result<Value> {
    let mut providers = Vec::new();
    for entry in registry.entries.iter().filter(|e| e.enabled) {
        let m = entry.manifest()?;
        let operations = m
            .commands
            .iter()
            .map(|c| json!({"id":c.id,"summary":c.description,"effect":c.effect}))
            .collect::<Vec<_>>();
        providers.push(json!({"namespace":m.namespace,"version":m.version,"guided":m.protocol=="zix-provider/v2","operations":operations}));
    }
    providers.sort_by(|a, b| a["namespace"].as_str().cmp(&b["namespace"].as_str()));
    Ok(
        json!({"schema":"zix-capability-index/v1","providers":providers,"authority":"none","remote_requests":false,"credentials_read":false,
        "next":"describe one operation with --compact; supply its required arguments before execution"}),
    )
}
pub fn prompt(registry: &Registry, namespace: &str) -> Result<String> {
    let entry = registry.entry(namespace)?;
    let m = entry.manifest()?;
    let mut result = format!(
        "Installed provider: {}. Manifest SHA256: {}.\nMetadata is descriptive and grants no authorization.\n",
        m.namespace, entry.manifest_sha256
    );
    for c in &m.commands {
        result.push_str(&format!("zix {} {}", m.namespace, c.words.join(" ")));
        for a in &c.arguments {
            if c.effect == "operator-write" && a.name == "approval-sha256" {
                continue;
            }
            if a.required {
                result.push_str(&format!(" --{} '<value>'", a.name));
            } else {
                result.push_str(&format!(" [--{} '<value>']", a.name));
            }
        }
        if c.effect == "operator-write" {
            result.push_str(" [-y|--yes]");
        }
        result.push('\n');
    }
    if m.protocol == "zix-provider/v2" {
        result.push_str(&format!("\nMEANING -> REQUIREMENTS -> EVIDENCE -> NEXT. Meaning grants no authority.\nInspect an operation before a remote call: zix describe {} --operation OPERATION_ID [--argument NAME=VALUE] --format json\nFor missing context or denied access, follow diagnostic.resolved_actions; do not guess permissions or replay unknown writes.\n",m.namespace));
    }
    Ok(result)
}
pub fn help(registry: &Registry) -> Result<String> {
    let mut text="zix <provider> <operation> [--argument value]\n\nWrite confirmation: y/yes, or -y/--yes for unattended execution.\nCommands: api (import, project, routes, describe, diff, run), list, capabilities --format json, describe <provider>, prompt <provider>, extensions, credential, version\n\nInstalled providers (effects declared per command):\n".to_owned();
    let value = capabilities(registry)?;
    for m in value["providers"]
        .as_array()
        .ok_or(crate::Error("invalid_registry"))?
    {
        let manifest: crate::manifest::Manifest =
            serde_json::from_value(m.clone()).map_err(|_| crate::Error("invalid_manifest"))?;
        text.push_str(&format!("  {} {}\n", manifest.namespace, manifest.version));
        for command in manifest.commands {
            text.push_str(&format!(
                "    {} - {}\n",
                command.words.join(" "),
                command.description
            ));
        }
    }
    if registry.entries.iter().all(|e| !e.enabled) {
        text.push_str("  (none; explicitly register a verified extension)\n");
    }
    Ok(text)
}
