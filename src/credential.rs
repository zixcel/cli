//! Local-owner credential composition. Crowsi owns storage and one-use leases.
use crate::{Error, Result};
mod bindings;
#[cfg(feature = "credential-store")]
mod custody;
#[cfg(feature = "credential-store")]
mod field;
mod options;
#[cfg(feature = "credential-store")]
mod terminal;
#[cfg(feature = "credential-store")]
mod use_secret;
pub(crate) use options::Request;

const HELP: &str = "zix credential status|unlock|lock --collection ALIAS
zix credential inspect|remove --collection ALIAS --reference TENANT/SERVICE/PURPOSE/AUDIENCE/ID
zix credential set --collection ALIAS --reference TENANT/SERVICE/PURPOSE/AUDIENCE/ID --host HOST
zix credential run --collection ALIAS --reference TENANT/SERVICE/PURPOSE/AUDIENCE/ID --host HOST --environment ZIX_NAME --manifest-sha256 HASH --executable-sha256 HASH -- NAMESPACE OPERATION [ARGUMENTS]

Set reads a bounded, hidden terminal prompt; no secret argv, files or raw get command.
zix credential run --collection ALIAS --bindings PRIVATE_JSON --manifest-sha256 HASH --executable-sha256 HASH -- NAMESPACE OPERATION [ARGUMENTS]\n\nRun selects one registered operation and supplies only its declared credential environment.\nUse -y/--yes for explicit unattended confirmation; provider flags may follow the command.\nSecret enrollment still uses the hidden terminal prompt.
Requires the Linux credential-store build, an existing persistent OS Secret Service collection,
and an explicitly authorized local-owner operation. Never creates or auto-unlocks a collection.
Lock affects the entire selected collection. Local removal does not revoke a provider token.";

pub(crate) async fn run(args: &[String]) -> Result<bool> {
    if args.is_empty() || args == ["--help"] {
        println!("{HELP}");
        return Ok(true);
    }
    let request = options::parse(args)?;
    #[cfg(feature = "credential-store")]
    {
        custody::run(request).await
    }
    #[cfg(not(feature = "credential-store"))]
    {
        let _ = request;
        Err(Error("credential_store_not_in_build"))
    }
}

pub(crate) fn validate_target(
    request: &Request,
    entry: &crate::registry::Entry,
    selected: &crate::Selected<'_>,
) -> Result<()> {
    let m = entry.manifest()?;

    let credentials = selected
        .command
        .environment
        .as_ref()
        .map_or(&m.credential_environment, |e| &e.credential_environment);
    if request.manifest_sha256 != entry.manifest_sha256
        || request.executable_sha256 != entry.executable_sha256
    {
        return Err(Error("credential_target_mismatch"));
    }
    for b in request.credentials() {
        if b.reference.split('/').nth(3) != Some(m.namespace.as_str())
            || !credentials.contains(&b.environment)
        {
            return Err(Error("credential_target_mismatch"));
        }
        if std::env::var_os(&b.environment).is_some() {
            return Err(Error("credential_environment_conflict"));
        }
    }
    Ok(())
}

pub(crate) fn resolve(
    request: &Request,
    entry: &crate::registry::Entry,
    selected: &crate::Selected<'_>,
) -> Result<Vec<(String, zeroize::Zeroizing<String>)>> {
    validate_target(request, entry, selected)?;
    #[cfg(feature = "credential-store")]
    {
        use_secret::resolve(request, entry, selected)
    }
    #[cfg(not(feature = "credential-store"))]
    {
        Err(Error("credential_store_not_in_build"))
    }
}
