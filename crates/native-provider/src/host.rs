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
mod request;
pub use request::validate_request;
pub fn serve(
    manifest: Manifest,
    handler: impl FnOnce(&Request) -> std::result::Result<Value, Failure>,
) -> ExitCode {
    finish(run(manifest, None, handler))
}
pub fn serve_guided(
    manifest: Manifest,
    catalog: zix_operation_contracts::guidance::Catalog,
    handler: impl FnOnce(&Request) -> std::result::Result<Value, Failure>,
) -> ExitCode {
    finish(run(manifest, Some(catalog), handler))
}
fn finish(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{{\"error\":\"{}\"}}", error.0);
            ExitCode::from(2)
        }
    }
}
fn run(
    manifest: Manifest,
    catalog: Option<zix_operation_contracts::guidance::Catalog>,
    handler: impl FnOnce(&Request) -> std::result::Result<Value, Failure>,
) -> Result<()> {
    manifest.validate()?;
    if let Some(catalog) = &catalog {
        crate::guidance::validate_catalog(&manifest, catalog)?;
        if manifest.protocol != "zix-provider/v2" {
            return Err(Error("guidance_protocol_required"));
        }
    } else if manifest.protocol != "zix-provider/v1" {
        return Err(Error("guidance_catalog_missing"));
    }
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--manifest"] {
        println!(
            "{}",
            serde_json::to_string(&manifest).map_err(|_| Error("invalid_manifest"))?
        );
        return Ok(());
    }
    if args == ["--guidance"] {
        let catalog = catalog.as_ref().ok_or(Error("guidance_catalog_missing"))?;
        println!(
            "{}",
            serde_json::to_string(catalog).map_err(|_| Error("invalid_guidance"))?
        );
        return Ok(());
    }
    let flag = if manifest.protocol == "zix-provider/v2" {
        "--zix-provider-v2"
    } else {
        "--zix-provider-v1"
    };
    if args != [flag] {
        return Err(Error("invalid_arguments"));
    }
    let bytes = files::bounded(stdin().lock(), 8192)?;
    let request: Request = serde_json::from_value(json::parse(&bytes, 8192)?)
        .map_err(|_| Error("invalid_provider_request"))?;
    validate_request(&manifest, &request)?;
    let mut response =
        json!({"schema":manifest.protocol,"id":request.id,"operation":request.operation});
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
    if let Some(catalog) = &catalog {
        response["diagnostic"] =
            serde_json::to_value(crate::guidance::diagnose(catalog, &request, &response)?)
                .map_err(|_| Error("invalid_diagnostic"))?;
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
