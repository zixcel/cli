use crate::{Error, Result, Selected, registry::Entry, sealed};
use crowsi_process_adapter::{Launch, OwnedProcess};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
async fn bounded(reader: impl AsyncRead + Unpin, maximum: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Error("provider_io_failed"))?;
    if bytes.len() > maximum {
        return Err(Error("provider_output_limit"));
    }
    Ok(bytes)
}
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
    let mut launch = Launch::new(&snapshot.path).args(["--zix-provider-v1"]);
    for name in &m.runtime_environment {
        let value = std::env::var(name).map_err(|_| Error("runtime_environment_missing"))?;
        if !value.starts_with("unix:path=/")
            || value.len() > 4096
            || value.bytes().any(|b| b.is_ascii_control())
            || value.contains(';')
        {
            return Err(Error("invalid_runtime_environment"));
        }
        launch = launch.env(name, value);
    }
    let mut secrets = Vec::new();
    for name in &m.credential_environment {
        if let Some(value) = std::env::var_os(name) {
            let text = value
                .to_str()
                .ok_or(Error("invalid_credential_environment"))?;
            if text.is_empty()
                || text.len() > 16384
                || !text.is_ascii()
                || text.bytes().any(|b| b.is_ascii_control())
            {
                return Err(Error("invalid_credential_environment"));
            }
            secrets.push(text.to_owned());
            launch = launch.env(name, value);
        }
    }
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
    result
}
