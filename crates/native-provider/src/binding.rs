use crate::{Error, Manifest, Request, Result};
pub(crate) fn validate(manifest: &Manifest, request: &Request) -> Result<()> {
    if manifest.schema == "zix-extension/v1" {
        if request.binding.is_some() {
            return Err(Error("unexpected_provider_binding"));
        }
        return Ok(());
    }
    let observed = request
        .binding
        .as_ref()
        .ok_or(Error("provider_binding_missing"))?;
    let executable =
        std::fs::File::open("/proc/self/exe").map_err(|_| Error("executable_unavailable"))?;
    let bytes = zix_cli::files::bounded(executable, 128 * 1024 * 1024)?;
    let catalog = serde_json::to_value(manifest).map_err(|_| Error("invalid_manifest"))?;
    let expected = zix_operation_contracts::Binding::for_catalog(
        &manifest.package,
        &manifest.version,
        &catalog,
        &zix_cli::files::digest(&bytes),
    );
    if observed != &expected {
        return Err(Error("provider_binding_mismatch"));
    }
    Ok(())
}
