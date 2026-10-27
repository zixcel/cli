use crate::{Error, Request, Result};
use serde_json::Value;
use zix_operation_contracts::guidance::{Catalog, Diagnostic};
pub fn diagnose(catalog: &Catalog, request: &Request, response: &Value) -> Result<Diagnostic> {
    let guide = catalog
        .select(&request.operation, &request.arguments)
        .ok_or(Error("guidance_operation_missing"))?
        .clone();
    let result = Diagnostic::observe(guide, response);
    if !result.validate() {
        return Err(Error("invalid_diagnostic"));
    }
    Ok(result)
}
