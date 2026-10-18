use crate::{Error, Failure, Provider, Request, Result, serve};
use crowsi_provider_read_http::{Error as HttpError, HttpsRead};
use serde_json::Value;
use std::{process::ExitCode, time::Duration};
use zeroize::Zeroizing;
fn execute(provider: &Provider, request: &Request) -> std::result::Result<Value, Failure> {
    let result = (|| -> Result<_> {
        let prepared = provider.prepare(request)?;
        let token = Zeroizing::new(
            std::env::var(&provider.policy.credential_environment)
                .map_err(|_| Error("credential_missing"))?,
        );
        Ok((prepared, token))
    })()
    .map_err(Failure::from)?;
    let (plan, token) = result;
    let mut http = HttpsRead::new(
        &plan.origin,
        &token,
        Duration::from_millis(request.limits.timeout_ms.saturating_sub(20).max(50)),
        1,
    )
    .map_err(convert)?;
    let headers = plan
        .headers
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect::<Vec<_>>();
    let bytes = http
        .get(&plan.url, &headers, 1024 * 1024)
        .map_err(convert)?;
    provider.project(request, &bytes).map_err(Failure::from)
}
fn convert(error: HttpError) -> Failure {
    let (code, http_status) = match error {
        HttpError::Status(s) => ("provider_http_failed", Some(s)),
        HttpError::BodyLimit => ("provider_response_limit", None),
        HttpError::Credential => ("credential_invalid", None),
        _ => ("provider_transport_failed", None),
    };
    Failure { code, http_status }
}
pub fn entry(routes: &[u8], policy: &[u8], version: &str) -> ExitCode {
    match Provider::load(routes, policy, version) {
        Ok(provider) => serve(provider.manifest().clone(), |request| {
            execute(&provider, request)
        }),
        Err(error) => {
            eprintln!("{{\"error\":\"{}\"}}", error.0);
            ExitCode::from(2)
        }
    }
}
