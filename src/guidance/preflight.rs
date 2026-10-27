use super::*;
use crate::Selected;
use zix_operation_contracts::guidance::Diagnostic;

pub async fn context(entry: &Entry, selected: &Selected<'_>) -> Result<Option<Value>> {
    if entry.manifest()?.protocol != "zix-provider/v2" {
        return Ok(None);
    }
    let catalog = super::fetch::catalog(entry).await?;
    let guide = catalog
        .select(&selected.command.id, &selected.arguments)
        .ok_or(Error("guidance_operation_missing"))?;
    if guide
        .required_arguments
        .iter()
        .all(|k| selected.arguments.contains_key(k))
    {
        return Ok(None);
    }
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error("clock_unavailable"))?
        .as_nanos();
    let id = crate::files::digest(format!("{}:{tick}:preflight", std::process::id()).as_bytes());
    let mut response = json!({"schema":"zix-provider/v2","id":&id[..32],
        "operation":selected.command.id,"error":"operation_context_missing"});
    response["diagnostic"] = serde_json::to_value(Diagnostic::observe(guide.clone(), &response))
        .map_err(|_| Error("invalid_diagnostic"))?;
    Ok(Some(response))
}
