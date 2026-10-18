use super::Format;
use crate::{Error, Result};
use serde_json::Value;
pub(super) fn metadata(root: &Value, format: &Format) -> Result<(String, String, Option<String>)> {
    let text = |v: &Value| {
        v.as_str()
            .map(str::to_owned)
            .ok_or(Error("invalid_api_metadata"))
    };
    match format {
        Format::Openapi => {
            let version = text(&root["openapi"])?;
            let parts = version.split('.').collect::<Vec<_>>();
            if parts.len() != 3
                || parts[0] != "3"
                || !["0", "1"].contains(&parts[1])
                || parts[2].is_empty()
                || !parts[2].bytes().all(|b| b.is_ascii_digit())
            {
                return Err(Error("unsupported_openapi_version"));
            }
            Ok((
                text(&root["info"]["title"])?,
                text(&root["info"]["version"])?,
                Some(version),
            ))
        }
        Format::Discovery => {
            if root["discoveryVersion"] != "v1" {
                return Err(Error("unsupported_discovery_version"));
            }
            let revision = root.get("revision").map(text).transpose()?;
            Ok((text(&root["name"])?, text(&root["version"])?, revision))
        }
    }
}
