use super::*;
use zix_operation_contracts::guidance::{Requirement, RequirementKind, Resolution};
pub(super) fn emit(entry: &Entry, response: &Value) -> Result<()> {
    let m = entry.manifest()?;
    let Some(c) = m.commands.iter().find(|c| response["operation"] == c.id) else {
        return Ok(());
    };
    let guide = Guide {
        operation: c.id.clone(),
        selectors: Default::default(),
        intent: c.id.replace('.', "_"),
        keywords: vec![m.namespace.to_uppercase(), "OPERATION".into()],
        requirements: c
            .required_permissions
            .iter()
            .map(|v| Requirement::new(RequirementKind::ProviderPermission, v.clone()))
            .collect(),
        next_actions: vec![],
        documentation: vec![],
    };
    let d = Diagnostic::observe(guide, response);
    if d.state == Resolution::NeedsResolution {
        eprintln!(
            "{}",
            json!({"schema":"zix-legacy-guidance/v1","diagnostic":d,"catalog_coverage":"legacy_manifest_only","inspect_argv":["zix","describe",m.namespace,"--format","json"],"authority":"none"})
        );
    }
    Ok(())
}
