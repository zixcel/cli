use crate::{Error, Result, Selected, registry::Entry, sealed};
use crowsi_process_adapter::OwnedProcess;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
mod environment;
mod output;
use output::bounded;
pub async fn dispatch(entry: &Entry, selected: Selected<'_>) -> Result<Value> {
    if !entry.enabled {
        return Err(Error("disabled"));
    }
    let m = entry.manifest()?;
    if !m.commands.contains(selected.command) {
        return Err(Error("operation_binding_mismatch"));
    }
    crate::operator::authorize(&m, &entry.executable_sha256, &selected)?;
    let snapshot = sealed::snapshot(entry)?;
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error("clock_unavailable"))?
        .as_nanos();
    let id =
        crate::files::digest(format!("{}:{tick}", std::process::id()).as_bytes())[..32].to_owned();
    let mut request = json!({"schema":"zix-provider/v1","id":id,"operation":selected.command.id,"arguments":selected.arguments,
        "limits":{"timeout_ms":m.timeout_ms,"maximum_response_bytes":131072}});
    if m.schema == "zix-extension/v2" {
        request["binding"] =
            serde_json::to_value(crate::operator::binding(&m, &entry.executable_sha256)?)
                .map_err(|_| Error("invalid_manifest"))?;
    }
    let (launch, secrets) = environment::launch(&snapshot.path, &m, selected.command)?;
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
        crate::response::response(&output, &id, &selected.command.id, &secrets)
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
            "{{\"cause\":\"{}\",\"reconciliation\":\"provider_readback_required\"}}",
            cause.0
        );
        return Err(Error("operator_write_outcome_unknown"));
    }
    result
}
