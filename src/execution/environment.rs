use crate::{
    Error, Result,
    manifest::{Command, Manifest},
};
use crowsi_process_adapter::Launch;
use std::path::Path;

pub(super) fn launch(
    path: &Path,
    manifest: &Manifest,
    command: &Command,
) -> Result<(Launch, Vec<String>)> {
    let mut launch = Launch::new(path).args(["--zix-provider-v1"]);
    let (runtime, credentials) = command.environment.as_ref().map_or(
        (
            &manifest.runtime_environment,
            &manifest.credential_environment,
        ),
        |scope| (&scope.runtime_environment, &scope.credential_environment),
    );
    for name in runtime {
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
    for name in credentials {
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
    Ok((launch, secrets))
}
