use super::*;
use crate::{execution::output::bounded, sealed};
use crowsi_process_adapter::{Launch, OwnedProcess};
use std::time::Duration;
use zix_operation_contracts::guidance::Catalog;
pub(super) async fn catalog(entry: &Entry) -> Result<Catalog> {
    let manifest = entry.manifest()?;
    if manifest.protocol != "zix-provider/v2" {
        return Err(Error("guidance_not_supported"));
    }
    let executable = sealed::snapshot(entry)?;
    let launch = Launch::new(&executable.path).args(["--guidance"]);
    let mut child = OwnedProcess::spawn(launch)
        .await
        .map_err(|_| Error("provider_start_failed"))?;
    drop(child.take_stdin());
    let output = child.take_stdout().ok_or(Error("provider_io_failed"))?;
    let errors = child.take_stderr().ok_or(Error("provider_io_failed"))?;
    let mut wait_started = false;
    let work = async {
        let (bytes, _) = tokio::try_join!(bounded(output, 65536), bounded(errors, 8192))?;
        wait_started = true;
        if child
            .wait()
            .await
            .map_err(|_| Error("provider_wait_failed"))?
            .code()
            != Some(0)
        {
            return Err(Error("guidance_unavailable"));
        }
        let catalog: Catalog = serde_json::from_value(crate::api_spec::json::parse(&bytes, 65536)?)
            .map_err(|_| Error("invalid_guidance"))?;
        if !catalog.validate()
            || catalog.namespace != manifest.namespace
            || catalog.package != manifest.package
            || catalog.version != manifest.version
        {
            return Err(Error("invalid_guidance"));
        }
        for g in &catalog.entries {
            let c = manifest
                .commands
                .iter()
                .find(|c| c.id == g.operation)
                .ok_or(Error("invalid_guidance_operation"))?;
            if g.required_arguments
                .iter()
                .any(|k| !c.arguments.iter().any(|a| a.name == *k))
                || g.selectors.iter().any(|(k, v)| {
                    !c.arguments
                        .iter()
                        .any(|a| a.name == *k && v.len() <= a.maximum)
                })
            {
                return Err(Error("invalid_guidance_selector"));
            }
        }
        Ok(catalog)
    };
    let result = tokio::select! {
        v=tokio::time::timeout(Duration::from_secs(5),work)=>v.map_err(|_|Error("guidance_timeout")).and_then(|v|v),
        _=tokio::signal::ctrl_c()=>Err(Error("cancelled")),
    };
    if result.is_err() {
        let _ = child.kill();
        if !wait_started {
            let _ = child.shutdown(Duration::ZERO).await;
        }
    }
    drop(executable.file);
    result
}
