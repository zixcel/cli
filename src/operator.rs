use crate::{Error, Result, Selected, manifest::Manifest};
use std::{
    io::{BufRead, IsTerminal, Read, Write},
    path::Path,
};
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
    selected: &Selected<'_>,
) -> Result<()> {
    if selected.command.effect != "operator-write" {
        return Ok(());
    }
    // Explicit human terminal flow only. This is not hostile same-user process isolation.
    if !std::io::stdin().is_terminal() || std::env::var_os("GITHUB_ACTIONS").is_some() {
        return Err(Error("interactive_authorization_required"));
    }
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
    let approved = arg("approval-sha256")?;
    if !p.matches(
        &selected.command.id,
        &binding(m, executable_sha256)?,
        approved,
    ) {
        return Err(Error("plan_binding_mismatch"));
    }
    eprintln!(
        "Local operator write: {} ({} {}). Review the plan file; unlocking is not approval.",
        p.operation, m.package, m.version
    );
    eprint!("Enter this reviewed plan's exact SHA256 (empty cancels): ");
    std::io::stderr()
        .flush()
        .map_err(|_| Error("approval_unavailable"))?;
    let mut input = String::new();
    std::io::stdin()
        .lock()
        .take(66)
        .read_line(&mut input)
        .map_err(|_| Error("approval_unavailable"))?;
    if input.trim() != approved {
        return Err(Error("approval_declined"));
    }
    Ok(())
}
