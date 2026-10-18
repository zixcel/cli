//! Offline API contracts are descriptive data, never executable authority.
mod diff;
mod discovery;
mod metadata;
pub use diff::diff;
mod json;
mod model;
mod openapi;
mod references;
mod validate;
use crate::{Error, Result, files};
pub use model::{Binding, Catalog, Format, Operation, Review, Selection, Source};
use std::collections::BTreeSet;
pub const SPEC_LIMIT: usize = 8 * 1024 * 1024;
pub fn import(source: &[u8], review: &[u8]) -> Result<Catalog> {
    if source.len() > SPEC_LIMIT {
        return Err(Error("api_input_too_large"));
    }
    let review: Review = serde_json::from_value(json::parse(review, 65536)?)
        .map_err(|_| Error("invalid_api_review"))?;
    if review.schema != "zix-api-review/v1"
        || review.source_sha256 != files::digest(source)
        || review.operations.is_empty()
        || review.operations.len() > 32
    {
        return Err(Error("api_source_or_review_mismatch"));
    }
    let root = json::parse(source, SPEC_LIMIT)?;
    let source_metadata = metadata::metadata(&root, &review.format, &review.source_sha256)?;
    let mut operations = Vec::new();
    let mut ids = BTreeSet::new();
    let mut encoded_size = 0;
    for selected in &review.operations {
        if !ids.insert(&selected.id) {
            return Err(Error("duplicate_api_selection"));
        }
        let operation = match review.format {
            Format::Openapi => openapi::select(&root, selected)?,
            Format::Discovery => discovery::select(&root, selected)?,
        };
        encoded_size += serde_json::to_vec(&operation)
            .map_err(|_| Error("invalid_api_catalog"))?
            .len();
        if encoded_size > SPEC_LIMIT {
            return Err(Error("api_catalog_too_large"));
        }
        operations.push(operation);
    }
    operations.sort_by(|a, b| a.id.cmp(&b.id));
    let catalog = Catalog {
        schema: "zix-api-catalog/v1".into(),
        source: source_metadata,
        operations,
    };
    catalog.validate()?;
    if serde_json::to_vec(&catalog)
        .map_err(|_| Error("invalid_api_catalog"))?
        .len()
        > SPEC_LIMIT
    {
        return Err(Error("api_catalog_too_large"));
    }
    Ok(catalog)
}
pub fn parse(bytes: &[u8]) -> Result<Catalog> {
    let catalog: Catalog = serde_json::from_value(json::parse(bytes, SPEC_LIMIT)?)
        .map_err(|_| Error("invalid_api_catalog"))?;
    catalog.validate()?;
    Ok(catalog)
}
