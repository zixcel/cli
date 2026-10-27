use super::fetch::catalog;
use super::*;
use std::collections::BTreeMap;
pub(crate) async fn describe(registry: &Registry, args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err(Error("invalid_arguments"));
    }
    let compact = args.iter().filter(|a| a.as_str() == "--compact").count();
    if compact > 1 {
        return Err(Error("invalid_arguments"));
    }
    let args = args
        .iter()
        .filter(|a| a.as_str() != "--compact")
        .cloned()
        .collect::<Vec<_>>();
    let entry = registry.entry(&args[1])?;
    let mut operation = None;
    let mut selectors: BTreeMap<String, String> = BTreeMap::new();
    for pair in args[2..].chunks(2) {
        if pair.len() != 2 {
            return Err(Error("invalid_arguments"));
        }
        match pair[0].as_str() {
            "--operation" if operation.is_none() => operation = Some(pair[1].clone()),
            "--argument" => {
                let (k, v) = pair[1].split_once('=').ok_or(Error("invalid_arguments"))?;
                if selectors.insert(k.into(), v.into()).is_some() {
                    return Err(Error("invalid_arguments"));
                }
            }
            "--format" if pair[1] == "json" => {}
            _ => return Err(Error("invalid_arguments")),
        }
    }
    let catalog = catalog(entry).await?;
    let manifest = entry.manifest()?;
    if let Some(operation) = operation {
        let command = manifest
            .commands
            .iter()
            .find(|c| c.id == operation)
            .ok_or(Error("unknown_operation"))?;
        if selectors.iter().any(|(k, v)| {
            !command
                .arguments
                .iter()
                .any(|a| a.name == *k && v.len() <= a.maximum)
        }) {
            return Err(Error("invalid_guidance_selector"));
        }
        let guide = catalog
            .select(&operation, &selectors)
            .ok_or(Error("guidance_operation_missing"))?;
        if compact == 1 {
            println!(
                "{}",
                super::brief::operation(registry, entry, &manifest, command, &catalog, guide)?
            );
            return Ok(());
        }
        let required = command
            .arguments
            .iter()
            .filter(|a| a.required && a.name != "approval-sha256")
            .map(|a| &a.name)
            .chain(guide.required_arguments.iter())
            .collect::<std::collections::BTreeSet<_>>();
        println!(
            "{}",
            json!({"schema":"zix-operation-guidance/v1","namespace":manifest.namespace,"version":manifest.version,"guide":guide,"effect":command.effect,"required_arguments":required,"resolved_actions":actions(registry,guide,None)?,"authority":"none","remote_requests":false,"credentials_read":false,"manifest_sha256":entry.manifest_sha256,"executable_sha256":entry.executable_sha256})
        );
    } else {
        if compact == 1 {
            return Err(Error("guidance_operation_required"));
        }
        println!(
            "{}",
            json!({"schema":"zix-provider-guidance/v1","catalog":catalog,"authority":"none","remote_requests":false,"credentials_read":false})
        );
    }
    Ok(())
}
