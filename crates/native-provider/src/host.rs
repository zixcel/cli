use crate::{Error, Manifest, Request, Result};
use serde_json::{Value, json};
use std::{io::stdin, process::ExitCode};
use zeroize::Zeroizing;
use zix_cli::{api_spec::json, files};
#[derive(Clone, Copy, Debug)]
pub struct Failure {
    pub code: &'static str,
    pub http_status: Option<u16>,
}
impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Self {
            code: error.0,
            http_status: None,
        }
    }
}
pub fn validate_request(manifest: &Manifest, request: &Request) -> Result<()> {
    if request.schema != "zix-provider/v1"
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
    Ok(())
}
pub fn serve(
    manifest: Manifest,
    handler: impl FnOnce(&Request) -> std::result::Result<Value, Failure>,
) -> ExitCode {
    match run(manifest, handler) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{{\"error\":\"{}\"}}", error.0);
            ExitCode::from(2)
        }
    }
}
fn run(
    manifest: Manifest,
    handler: impl FnOnce(&Request) -> std::result::Result<Value, Failure>,
) -> Result<()> {
    manifest.validate()?;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--manifest"] {
        println!(
            "{}",
            serde_json::to_string(&manifest).map_err(|_| Error("invalid_manifest"))?
        );
        return Ok(());
    }
    if args != ["--zix-provider-v1"] {
        return Err(Error("invalid_arguments"));
    }
    let bytes = files::bounded(stdin().lock(), 8192)?;
    let request: Request = serde_json::from_value(json::parse(&bytes, 8192)?)
        .map_err(|_| Error("invalid_provider_request"))?;
    validate_request(&manifest, &request)?;
    let mut response =
        json!({"schema":"zix-provider/v1","id":request.id,"operation":request.operation});
    match handler(&request) {
        Ok(value) => response["result"] = value,
        Err(error) => {
            let code = if !error.code.is_empty()
                && error.code.len() <= 64
                && error
                    .code
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'_')
            {
                error.code
            } else {
                "provider_failed"
            };
            response["error"] = json!(code);
            if let Some(status) = error.http_status.filter(|s| (100..=599).contains(s)) {
                response["http_status"] = json!(status);
            }
        }
    }
    let secrets = Zeroizing::new(
        manifest
            .credential_environment
            .iter()
            .filter_map(|n| std::env::var(n).ok())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>(),
    );
    if zix_cli::response::contains_secret(&response, &secrets) {
        return Err(Error("credential_rejected"));
    }
    let encoded =
        serde_json::to_string(&response).map_err(|_| Error("invalid_provider_response"))?;
    if encoded.len() + 1 > request.limits.maximum_response_bytes {
        return Err(Error("provider_output_limit"));
    }
    println!("{encoded}");
    Ok(())
}
