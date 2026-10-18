use super::{Review, SPEC_LIMIT, json};
use crate::{Error, Result, files};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use url::Url;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    documents: BTreeMap<String, String>,
}
pub(super) struct Documents<'a> {
    pub root: &'a Value,
    pub source_uri: String,
    pub documents: BTreeMap<String, Value>,
    pub reference_limit: usize,
}
pub(super) fn uri(value: &str) -> Result<Url> {
    let url = Url::parse(value).map_err(|_| Error("invalid_api_document_uri"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.as_str() != value
        || url.query_pairs().any(|(k, _)| {
            let k = k.to_ascii_lowercase();
            k.contains("token")
                || k.contains("secret")
                || k.contains("password")
                || ["key", "sig", "signature", "authorization"].contains(&k.as_str())
        })
    {
        return Err(Error("invalid_api_document_uri"));
    }
    Ok(url)
}
impl<'a> Documents<'a> {
    pub fn load(
        root: &'a Value,
        review: &Review,
        bundle: Option<&[u8]>,
        project: bool,
    ) -> Result<Self> {
        let reference_limit = review.reference_limit.unwrap_or(128);
        if reference_limit == 0
            || reference_limit > if project { 1024 } else { 128 }
            || review.reference_sources.len() > 16
        {
            return Err(Error("api_reference_limit"));
        }
        let source_uri = review.source_uri.clone().unwrap_or_default();
        if !source_uri.is_empty() {
            uri(&source_uri)?;
        }
        let bundle = match bundle {
            Some(bytes) => {
                serde_json::from_value::<Bundle>(json::parse(bytes, SPEC_LIMIT)?)
                    .map_err(|_| Error("invalid_api_document_bundle"))?
                    .documents
            }
            None => BTreeMap::new(),
        };
        if bundle.len() != review.reference_sources.len()
            || (!bundle.is_empty() && source_uri.is_empty())
        {
            return Err(Error("api_document_bundle_mismatch"));
        }
        let mut documents = BTreeMap::new();
        for (url, raw) in bundle {
            uri(&url)?;
            if url == source_uri
                || review.reference_sources.get(&url) != Some(&files::digest(raw.as_bytes()))
            {
                return Err(Error("api_document_integrity_mismatch"));
            }
            documents.insert(url, json::parse(raw.as_bytes(), SPEC_LIMIT)?);
        }
        Ok(Self {
            root,
            source_uri,
            documents,
            reference_limit,
        })
    }
    pub fn value(&self, document: &str, pointer: &str) -> Result<&Value> {
        let document = if document == self.source_uri {
            self.root
        } else {
            self.documents
                .get(document)
                .ok_or(Error("unreviewed_api_document"))?
        };
        document
            .pointer(pointer)
            .ok_or(Error("missing_api_reference"))
    }
}
