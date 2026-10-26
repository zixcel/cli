use crate::{
    Error, Result,
    registry::{Entry, Registry},
};
use serde_json::{Value, json};
use std::path::Path;
fn metadata(entry: &Entry) -> Result<Value> {
    let m = entry.manifest()?;
    Ok(
        json!({"namespace":m.namespace,"package":m.package,"version":m.version,
        "enabled":entry.enabled,"platform":m.platform,"protocol":m.protocol,
        "manifest_sha256":entry.manifest_sha256,"executable_sha256":entry.executable_sha256,
        "executable":entry.executable,"manifest":m}),
    )
}
/// Reads metadata only: missing/tampered binaries remain inspectable for repair.
pub fn run(path: &Path, args: &[String]) -> Result<()> {
    let registry = Registry::load(path)?;
    let (name, json_output) = match args {
        [command] if command == "list" => (None, false),
        [command, flag, format] if command == "list" && flag == "--format" && format == "json" => {
            (None, true)
        }
        [command, name] if command == "inspect" => (Some(name.as_str()), false),
        [command, name, flag, format]
            if command == "inspect" && flag == "--format" && format == "json" =>
        {
            (Some(name.as_str()), true)
        }
        _ => return Err(Error("invalid_arguments")),
    };
    let mut entries = if let Some(name) = name {
        vec![metadata(registry.registered(name)?)?]
    } else {
        registry
            .entries
            .iter()
            .map(metadata)
            .collect::<Result<Vec<_>>>()?
    };
    entries.sort_by(|a, b| a["namespace"].as_str().cmp(&b["namespace"].as_str()));
    if json_output {
        let result = if name.is_some() {
            json!({"schema":"zix-extension-registration/v1","entry":entries[0]})
        } else {
            json!({"schema":"zix-extension-inventory/v1","entries":entries})
        };
        println!("{result}");
    } else {
        for entry in entries {
            println!(
                "{} {} {} {}",
                entry["namespace"].as_str().unwrap_or(""),
                entry["package"].as_str().unwrap_or(""),
                entry["version"].as_str().unwrap_or(""),
                if entry["enabled"] == true {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
    }
    Ok(())
}
