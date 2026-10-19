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
            if a.required {
                result.push_str(&format!(" --{} '<value>'", a.name));
            } else {
                result.push_str(&format!(" [--{} '<value>']", a.name));
            }
        }
        result.push('\n');
    }
    Ok(result)
}
pub fn help(registry: &Registry) -> Result<String> {
    let mut text="zix <provider> <operation> [--argument value]\n\nCommands: api (import, project, routes, describe, diff, run), list, capabilities --format json, describe <provider>, prompt <provider>, extensions, version\n\nInstalled providers (effects declared per command):\n".to_owned();
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
