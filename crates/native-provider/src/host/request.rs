use crate::{Error, Manifest, Request, Result};
pub fn validate_request(manifest: &Manifest, request: &Request) -> Result<()> {
    if request.schema != manifest.protocol
        || request.id.len() != 32
        || !request
            .id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || !(50..=manifest.timeout_ms).contains(&request.limits.timeout_ms)
        || !(1024..=131072).contains(&request.limits.maximum_response_bytes)
    {
        return Err(Error("invalid_provider_request"));
    }
    let command = manifest
        .commands
        .iter()
        .find(|c| c.id == request.operation)
        .ok_or(Error("unknown_operation"))?;
    let mut args = command.words.clone();
    for (name, value) in &request.arguments {
        args.extend([format!("--{name}"), value.clone()]);
    }
    zix_cli::select(manifest, &args)?;
    crate::binding::validate(manifest, request)?;
    Ok(())
}
