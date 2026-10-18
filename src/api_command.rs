use crate::{
    Error, Result, api_spec, execution, files,
    registry::{self, Registry},
    select,
};
use std::path::Path;
fn read(path: &str) -> Result<Vec<u8>> {
    files::read(Path::new(path), api_spec::SPEC_LIMIT, false)
}
fn catalog(path: &str) -> Result<api_spec::Catalog> {
    api_spec::parse(&read(path)?)
}
fn print(value: &impl serde::Serialize) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|_| Error("invalid_api_json"))?
    );
    Ok(())
}
pub async fn run(args: &[String]) -> Result<bool> {
    match args.first().map(String::as_str) {
        None | Some("help" | "--help") if args.len() <= 1 => {
            print(
                &serde_json::json!({"schema":"zix-api-help/v1", "authority":"none",
                "commands":["import SPEC REVIEW", "describe CATALOG", "diff OLD NEW",
                    "run CATALOG REVIEWED_SHA256 OPERATION [provider flags]"],
                "formats":["OpenAPI 3.0/3.1 JSON", "Google Discovery v1 JSON"],
                "execution":"explicit registered read-only provider binding"}),
            )?;
        }
        Some("import") if args.len() == 3 => {
            print(&api_spec::import(&read(&args[1])?, &read(&args[2])?)?)?;
        }
        Some("describe") if args.len() == 2 => print(&catalog(&args[1])?)?,
        Some("diff") if args.len() == 3 => {
            let value = api_spec::diff(&catalog(&args[1])?, &catalog(&args[2])?);
            print(&value)?;
            return Ok(value["equal"] == true);
        }
        Some("run") if args.len() >= 4 => {
            return dispatch(&args[1], &args[2], &args[3], &args[4..]).await;
        }
        _ => return Err(Error("invalid_api_arguments")),
    }
    Ok(true)
}
async fn dispatch(path: &str, sha256: &str, id: &str, args: &[String]) -> Result<bool> {
    let bytes = read(path)?;
    if !files::valid_digest(sha256) || files::digest(&bytes) != sha256 {
        return Err(Error("api_catalog_integrity_mismatch"));
    }
    let catalog = api_spec::parse(&bytes)?;
    let operation = catalog
        .operations
        .iter()
        .find(|o| o.id == id)
        .ok_or(Error("unknown_api_operation"))?;
    let binding = operation
        .binding
        .as_ref()
        .ok_or(Error("api_operation_not_bound"))?;
    let registry = Registry::load(&registry::location()?)?;
    let entry = registry.entry(&binding.namespace)?;
    if entry.manifest_sha256 != binding.manifest_sha256 {
        return Err(Error("api_provider_drift"));
    }
    let manifest = entry.manifest()?;
    let command = manifest
        .commands
        .iter()
        .find(|c| c.id == binding.command_id)
        .ok_or(Error("api_binding_command_missing"))?;
    if command.effect != "read" {
        return Err(Error("api_binding_effect_unsupported"));
    }
    let mut arguments = command.words.clone();
    arguments.extend_from_slice(args);
    let response = execution::dispatch(entry, select(&manifest, &arguments)?).await?;
    let success = response.get("error").is_none();
    print(&response)?;
    Ok(success)
}
