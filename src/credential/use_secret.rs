use super::{Request, custody};
use crate::{Error, Result, Selected, registry::Entry};
use crowsi_credential_broker::{AccessRequest, Broker, LeaseBinding};
use std::sync::Arc;
use zeroize::Zeroizing;

pub(super) fn resolve(
    request: &Request,
    entry: &Entry,
    selected: &Selected<'_>,
) -> Result<Vec<(String, Zeroizing<String>)>> {
    request
        .credentials()
        .iter()
        .map(|credential| {
            resolve_one(request, credential, entry, selected)
                .map(|s| (credential.environment.clone(), s))
        })
        .collect()
}
fn resolve_one(
    request: &Request,
    credential: &super::bindings::Binding,
    entry: &Entry,
    selected: &Selected<'_>,
) -> Result<Zeroizing<String>> {
    let reference = crowsi_credential_broker::SecretRef::parse_canonical(&credential.reference)
        .map_err(custody::error)?;
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
    let access = AccessRequest::new(reference.clone(), audience, &credential.host, 30)
        .map_err(custody::error)?;
    let grant = broker
        .issue_bound(access, binding.clone())
        .map_err(custody::error)?;
    let used = broker
        .consume_bound(grant.token(), audience, &credential.host, &binding)
        .map_err(custody::error)?;
    used.secret
        .expose(|bytes| super::field::select(bytes, credential.field.as_deref()))
}
