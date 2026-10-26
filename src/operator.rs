use crate::{Error, Result, Selected, manifest::Manifest};
use std::path::Path;
use zix_operation_contracts::{Binding, LocalPlan};
pub(crate) fn binding(m: &Manifest, executable_sha256: &str) -> Result<Binding> {
    let binding = Binding::for_catalog(
        &m.package,
        &m.version,
        &serde_json::to_value(m).map_err(|_| Error("invalid_manifest"))?,
        executable_sha256,
    );
    Ok(binding)
}
pub(crate) fn authorize(
    m: &Manifest,
    executable_sha256: &str,
    selected: &mut Selected<'_>,
) -> Result<()> {
    if selected.command.effect != "operator-write" {
        return Ok(());
    }
    crate::confirmation::require_interactive(selected.assume_yes)?;
    let arg = |name| {
        selected
            .arguments
            .get(name)
            .ok_or(Error("missing_argument"))
    };
    use std::os::unix::fs::PermissionsExt;
    let path = Path::new(arg("plan-file")?);
    let metadata = std::fs::symlink_metadata(path).map_err(|_| Error("unsafe_plan_file"))?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(Error("unsafe_plan_file"));
    }
    let bytes = crate::files::read(path, 1048576, false)?;
    let p: LocalPlan = serde_json::from_slice(&bytes).map_err(|_| Error("invalid_local_plan"))?;
    let digest = p.digest();
    let approved = selected
        .arguments
        .get("approval-sha256")
        .map(String::as_str)
        .unwrap_or(&digest);
    if !p.matches(
        &selected.command.id,
        &binding(m, executable_sha256)?,
        approved,
    ) {
        return Err(Error("plan_binding_mismatch"));
    }
    eprintln!(
        "Operation: {} ({} {})\nPlan: {}",
        p.operation,
        m.package,
        m.version,
        path.display()
    );
    if let Some(change) = p
        .plan
        .get("change")
        .or_else(|| p.plan.pointer("/original/change"))
    {
        eprintln!("Change: {change}");
    }
    crate::confirmation::ask(selected.assume_yes)?;
    selected.arguments.insert("approval-sha256".into(), digest);
    Ok(())
}
