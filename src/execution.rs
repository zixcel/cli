use crate::{Error, Result, Selected, registry::Entry, sealed};
use crowsi_process_adapter::OwnedProcess;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
mod environment;
pub(crate) mod output;
use output::bounded;
pub async fn dispatch(entry: &Entry, selected: Selected<'_>) -> Result<Value> {
    dispatch_inner(entry, selected, None).await
}
#[cfg(feature = "credential-store")]
pub(crate) async fn dispatch_credential(
    entry: &Entry,
    selected: Selected<'_>,
    credential: &crate::credential::Request,
) -> Result<Value> {
    dispatch_inner(entry, selected, Some(credential)).await
}
async fn dispatch_inner(
    entry: &Entry,
    mut selected: Selected<'_>,
    credential: Option<&crate::credential::Request>,
) -> Result<Value> {
    if !entry.enabled {
        return Err(Error("disabled"));
    }
    let m = entry.manifest()?;
    if !m.commands.contains(selected.command) {
        return Err(Error("operation_binding_mismatch"));
    }
    if let Some(response) = crate::guidance::preflight(entry, &selected).await? {
        return Ok(response);
    }
    crate::operator::authorize(&m, &entry.executable_sha256, &mut selected)?;
    let snapshot = sealed::snapshot(entry)?;
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error("clock_unavailable"))?
        .as_nanos();
    let id =
        crate::files::digest(format!("{}:{tick}", std::process::id()).as_bytes())[..32].to_owned();
    let mut request = json!({"schema":m.protocol,"id":id,"operation":selected.command.id,"arguments":selected.arguments,
        "limits":{"timeout_ms":m.timeout_ms,"maximum_response_bytes":131072}});
    if m.schema == "zix-extension/v2" {
        request["binding"] =
            serde_json::to_value(crate::operator::binding(&m, &entry.executable_sha256)?)
                .map_err(|_| Error("invalid_manifest"))?;
    }
    let extra = credential
        .map(|c| crate::credential::resolve(c, entry, &selected))
        .transpose()?;
    let (mut launch, mut secrets) = environment::launch(&snapshot.path, &m, selected.command)?;
    if let Some(values) = extra.as_ref() {
        for (environment, value) in values {
            launch = launch.env(environment, value.as_str());
            secrets.push(value.to_string());
        }
    }
    let secrets = zeroize::Zeroizing::new(secrets);
    let mut child = OwnedProcess::spawn(launch)
        .await
        .map_err(|_| Error("provider_start_failed"))?;
    let mut input = child.take_stdin().ok_or(Error("provider_io_failed"))?;
    let output = child.take_stdout().ok_or(Error("provider_io_failed"))?;
    let errors = child.take_stderr().ok_or(Error("provider_io_failed"))?;
    let mut wait_started = false;
    let work = async {
        input
            .write_all(&serde_json::to_vec(&request).map_err(|_| Error("protocol_rejected"))?)
            .await
            .map_err(|_| Error("provider_io_failed"))?;
        input
            .shutdown()
            .await
            .map_err(|_| Error("provider_io_failed"))?;
        drop(input);
        let (output, _errors) = tokio::try_join!(bounded(output, 131072), bounded(errors, 8192))?;
        wait_started = true;
        if child
            .wait()
            .await
            .map_err(|_| Error("provider_wait_failed"))?
            .code()
            != Some(0)
        {
            return Err(Error("provider_failed"));
        }
        let value = crate::response::response_with_protocol(
            &output,
            &id,
            &selected.command.id,
            &secrets,
            &m.protocol,
        )?;
        if m.protocol == "zix-provider/v2" {
            let d: zix_operation_contracts::guidance::Diagnostic =
                serde_json::from_value(value["diagnostic"].clone())
                    .map_err(|_| Error("invalid_diagnostic"))?;
            if d.guide
                .selectors
                .iter()
                .any(|(k, v)| selected.arguments.get(k) != Some(v))
            {
                return Err(Error("diagnostic_selector_mismatch"));
            }
        }
        Ok(value)
    };
    let result = tokio::select! {
        value=tokio::time::timeout(Duration::from_millis(m.timeout_ms),work)=>value.map_err(|_|Error("provider_timeout")).and_then(|r|r),
        _=tokio::signal::ctrl_c()=>Err(Error("cancelled")),
    };
    if result.is_err() {
        let _ = child.kill();
        // Do not repoll a possibly failed consumed waiter in the released adapter.
        if !wait_started {
            let _ = child.shutdown(Duration::ZERO).await;
        }
    }
    drop(snapshot.file);
    if let Err(cause) = result
        && selected.command.effect == "operator-write"
    {
        eprintln!(
            "{}",
            json!({"cause":cause.0,"namespace":m.namespace,"operation":selected.command.id,"reconciliation":"provider_readback_required","retry":"no_automatic_retry"})
        );
        return Err(Error("operator_write_outcome_unknown"));
    }
    result
}
