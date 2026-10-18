use super::{SPEC_LIMIT, json::Unique};
use crate::{Error, Result};
use serde_json::Value;
pub(super) fn parse(bytes: &[u8], limit: usize) -> Result<Value> {
    if bytes.len() > limit {
        return Err(Error("api_input_too_large"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Error("invalid_api_yaml"))?;
    let maximum = if limit > SPEC_LIMIT {
        2_000_000
    } else {
        200_000
    };
    let options = serde_saphyr::options! {
        strict_booleans:true,
        legacy_octal_numbers:false,
        reject_unsupported_tags:true,
        duplicate_keys:serde_saphyr::DuplicateKeyPolicy::Error,
        merge_keys:serde_saphyr::MergeKeyPolicy::Error,
        budget:serde_saphyr::budget! {
            max_depth:64,max_documents:1,max_nodes:maximum,max_events:maximum*4,
            max_anchors:0,max_aliases:0,max_merge_keys:0,max_inclusion_depth:0,
            max_total_scalar_bytes:limit,
        },
    };
    let Unique(value) = serde_saphyr::from_str_with_options(text, options)
        .map_err(|_| Error("invalid_api_yaml"))?;
    Ok(value)
}
