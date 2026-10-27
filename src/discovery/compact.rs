use crate::{Error, Result, registry::Registry};
use serde_json::json;
use std::collections::BTreeSet;
pub(crate) fn run(registry: &Registry, args: &[String]) -> Result<()> {
    if args.iter().filter(|a| a.as_str() == "--compact").count() != 1 {
        return Err(Error("invalid_arguments"));
    }
    let remaining = args
        .iter()
        .filter(|a| a.as_str() != "--compact")
        .collect::<Vec<_>>();
    let mut namespace = None;
    let mut format = false;
    for pair in remaining.chunks(2) {
        if pair.len() != 2 {
            return Err(Error("invalid_arguments"));
        }
        match pair[0].as_str() {
            "--namespace" if namespace.is_none() => namespace = Some(pair[1].as_str()),
            "--format" if !format && pair[1] == "json" => format = true,
            _ => return Err(Error("invalid_arguments")),
        }
    }
    if let Some(n) = namespace {
        registry.entry(n)?;
    }
    let mut providers = Vec::new();
    for entry in registry.entries.iter().filter(|e| e.enabled) {
        let m = entry.manifest()?;
        if namespace.is_some_and(|n| n != m.namespace) {
            continue;
        }
        let mut p = json!({"namespace":m.namespace,"version":m.version,"guided":m.protocol=="zix-provider/v2",
            "operation_count":m.commands.len(),"effects":m.commands.iter().map(|c|&c.effect).collect::<BTreeSet<_>>()});
        let topics = m
            .commands
            .iter()
            .filter_map(|c| c.description.split_once(':').map(|v| v.0))
            .flat_map(|s| s.split_ascii_whitespace())
            .filter(|s| s.len() <= 32)
            .collect::<BTreeSet<_>>();
        p["topics"] = json!(topics.into_iter().take(12).collect::<Vec<_>>());
        if namespace.is_some() {
            p["operations"] = json!(
                m.commands
                    .iter()
                    .map(|c| json!({"id":c.id,"summary":c.description,"effect":c.effect}))
                    .collect::<Vec<_>>()
            );
        } else {
            p["inspect_argv"] = json!([
                "zix",
                "capabilities",
                "--namespace",
                m.namespace,
                "--compact",
                "--format",
                "json"
            ]);
        }
        providers.push(p);
    }
    providers.sort_by(|a, b| a["namespace"].as_str().cmp(&b["namespace"].as_str()));
    println!(
        "{}",
        json!({"schema":"zix-capability-index/v1","scope":if namespace.is_some(){"operations"}else{"providers"},
        "providers":providers,"authority":"none","remote_requests":false,"credentials_read":false,
        "next":"select one namespace, then describe one operation with --compact; resolve named inputs before execution"})
    );
    Ok(())
}
