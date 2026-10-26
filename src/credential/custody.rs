use super::{Request, terminal};
use crate::{Error, Result};
use crowsi_credential_broker::{
    BrokerError, CredentialEntry, CredentialStore, LinuxCustodyStore, SecretRef, SecretValue,
};
use serde_json::json;

pub(super) fn error(cause: BrokerError) -> Error {
    Error(match cause {
        BrokerError::NotFound => "credential_not_found",
        BrokerError::BackendDenied => "credential_store_locked_or_denied",
        BrokerError::BackendUnavailable => "credential_store_unavailable",
        BrokerError::CredentialChanged => "credential_changed",
        _ => "credential_operation_rejected",
    })
}
pub(super) fn connect(alias: &str) -> Result<LinuxCustodyStore> {
    if let Some(address) = std::env::var_os("DBUS_SESSION_BUS_ADDRESS") {
        let address = address
            .to_str()
            .ok_or(Error("invalid_runtime_environment"))?;
        if !address.starts_with("unix:path=/")
            || address.len() > 4096
            || address.contains(';')
            || address.bytes().any(|b| b.is_ascii_control())
        {
            return Err(Error("invalid_runtime_environment"));
        }
    }
    LinuxCustodyStore::connect(alias).map_err(error)
}
pub(super) fn reference(request: &Request) -> Result<SecretRef> {
    SecretRef::parse_canonical(&request.reference).map_err(error)
}
pub(super) async fn run(request: Request) -> Result<bool> {
    if request.action == "run" {
        let registry = crate::registry::Registry::load(&crate::registry::location()?)?;
        let entry = registry.entry(&request.command[0])?;
        let manifest = entry.manifest()?;
        let selected = crate::select(&manifest, &request.command[1..])?;
        super::validate_target(&request, entry, &selected)?;
        let response = crate::execution::dispatch_credential(entry, selected, &request).await?;
        let success = response.get("error").is_none();
        println!("{response}");
        return Ok(success);
    }
    if matches!(
        request.action.as_str(),
        "set" | "remove" | "unlock" | "lock"
    ) && (!request.assume_yes || request.action == "set")
    {
        terminal::require_owner_terminal()?;
    }
    let store = connect(&request.collection)?;
    let result = match request.action.as_str() {
        "status" => json!({"locked": store.is_locked().map_err(error)?}),
        "unlock" => {
            store.unlock().map_err(error)?;
            json!({"locked":store.is_locked().map_err(error)?})
        }
        "lock" => {
            store.lock().map_err(error)?;
            json!({"locked":store.is_locked().map_err(error)?})
        }
        "inspect" => {
            let m = store.metadata(&reference(&request)?).map_err(error)?;
            json!({"reference":request.reference,"revision":m.revision().as_str(),"allowed_hosts":m.allowed_hosts()})
        }
        "set" => {
            // Prompt only after checking provider availability and lock state.
            if store.is_locked().map_err(error)? {
                return Err(Error("credential_store_locked_or_denied"));
            }
            let r = reference(&request)?;
            let replacing = match store.metadata(&r) {
                Ok(_) => true,
                Err(BrokerError::NotFound) => false,
                Err(e) => return Err(error(e)),
            };
            terminal::confirm(
                &format!(
                    "{} {} in {} for {}",
                    if replacing { "Replace" } else { "Store" },
                    request.reference,
                    request.collection,
                    request.host
                ),
                request.assume_yes,
            )?;
            let secret = SecretValue::new(terminal::read_secret()?.to_vec()).map_err(error)?;
            let entry = CredentialEntry::new(r, [request.host], secret).map_err(error)?;
            let revision = entry.metadata().revision().as_str().to_owned();
            store.put(entry).map_err(error)?;
            json!({"reference":request.reference,"revision":revision,"stored":true})
        }
        "remove" => {
            let r = reference(&request)?;
            terminal::confirm(
                &format!(
                    "Remove {} from {} (remote token remains valid)",
                    request.reference, request.collection
                ),
                request.assume_yes,
            )?;
            store.delete(&r).map_err(error)?;
            json!({"reference":request.reference,"removed":true,"remote_revoked":false})
        }
        _ => return Err(Error("invalid_arguments")),
    };
    println!(
        "{}",
        json!({"schema":"zix-credential/v1","backend":"crowsi-linux-secret-service",
        "collection":request.collection,"result":result})
    );
    Ok(true)
}
