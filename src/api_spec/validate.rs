use super::Catalog;
use crate::{Error, Result, files, manifest};
use std::collections::BTreeSet;
fn clean(text: &str, maximum: usize) -> bool {
    !text.is_empty() && text.len() <= maximum && !text.chars().any(char::is_control)
}
impl Catalog {
    pub fn validate(&self) -> Result<()> {
        let mut ids = BTreeSet::new();
        if self.schema != "zix-api-catalog/v1"
            || !files::valid_digest(&self.source.sha256)
            || !clean(&self.source.specification_version, 32)
            || !clean(&self.source.api, 256)
            || !clean(&self.source.version, 128)
            || self
                .source
                .revision
                .as_ref()
                .is_some_and(|v| !clean(v, 128))
            || self.operations.is_empty()
            || self.operations.len() > 32
        {
            return Err(Error("invalid_api_catalog"));
        }
        for o in &self.operations {
            if !clean(&o.id, 160)
                || !ids.insert(&o.id)
                || ![
                    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "TRACE",
                ]
                .contains(&o.method.as_str())
                || !clean(&o.path, 2048)
                || !o.contract.is_object()
                || !o.context.is_object()
                || o.references.len() > 128
            {
                return Err(Error("invalid_api_operation"));
            }
            if let Some(b) = &o.binding
                && (!manifest::token(&b.namespace)
                    || b.namespace == "api"
                    || !clean(&b.command_id, 100)
                    || !files::valid_digest(&b.manifest_sha256))
            {
                return Err(Error("invalid_api_binding"));
            }
        }
        Ok(())
    }
}
