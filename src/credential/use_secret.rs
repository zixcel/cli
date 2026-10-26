use super::{Request, custody};
use crate::{Error, Result, Selected, registry::Entry};
use crowsi_credential_broker::{AccessRequest, Broker, LeaseBinding};
use std::sync::Arc;
use zeroize::Zeroizing;

pub(super) fn resolve(
    request: &Request,
    entry: &Entry,
    selected: &Selected<'_>,
) -> Result<Zeroizing<String>> {
    let reference = custody::reference(request)?;
    let audience = reference.scope().audience();
    let broker =
        Broker::new(Arc::new(custody::connect(&request.collection)?)).map_err(custody::error)?;
    // This is explicit same-user local CLI authorization, not an identity assertion
    // or an authority-issued grant. The selected immutable operation is the workload.
    let arguments =
        serde_json::to_vec(&selected.arguments).map_err(|_| Error("invalid_arguments"))?;
    let binding = LeaseBinding::new(
        &format!("local-uid-{}", rustix::process::getuid().as_raw()),
        "local-cli",
        &format!("spiffe://zix.local/executable/{}", entry.executable_sha256),
        &crate::files::digest(&arguments),
        &entry.manifest_sha256,
        &crate::files::digest(selected.command.id.as_bytes()),
        "local-owner-invocation",
    )
    .map_err(custody::error)?;
    let access = AccessRequest::new(reference.clone(), audience, &request.host, 30)
        .map_err(custody::error)?;
    let grant = broker
        .issue_bound(access, binding.clone())
        .map_err(custody::error)?;
    let used = broker
        .consume_bound(grant.token(), audience, &request.host, &binding)
        .map_err(custody::error)?;
    used.secret.expose(|bytes| {
        if bytes.is_empty()
            || bytes.len() > 16384
            || !bytes.is_ascii()
            || bytes.iter().any(u8::is_ascii_control)
        {
            return Err(Error("invalid_credential_environment"));
        }
        Ok(Zeroizing::new(
            std::str::from_utf8(bytes)
                .map_err(|_| Error("invalid_credential_environment"))?
                .to_owned(),
        ))
    })
}
