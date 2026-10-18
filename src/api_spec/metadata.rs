use super::{Format, Source};
use crate::{Error, Result};
use serde_json::Value;
pub(super) fn metadata(root: &Value, format: &Format, sha256: &str) -> Result<Source> {
    let text = |v: &Value| {
        v.as_str()
            .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
            .map(str::to_owned)
            .ok_or(Error("invalid_api_metadata"))
    };
    match format {
        Format::Openapi | Format::OpenapiYaml => {
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
            Ok(Source {
                format: format.clone(),
                sha256: sha256.into(),
                api: text(&root["info"]["title"])?,
                version: text(&root["info"]["version"])?,
                source_uri: None,
                reference_sources: Default::default(),
                specification_version: version,
                revision: None,
            })
        }
        Format::Swagger => {
            if root["swagger"] != "2.0" {
                return Err(Error("unsupported_swagger_version"));
            }
            Ok(Source {
                format: format.clone(),
                sha256: sha256.into(),
                api: text(&root["info"]["title"])?,
                version: text(&root["info"]["version"])?,
                source_uri: None,
                reference_sources: Default::default(),
                specification_version: "2.0".into(),
                revision: None,
            })
        }
        Format::Discovery => {
            if root["discoveryVersion"] != "v1" {
                return Err(Error("unsupported_discovery_version"));
            }
            let revision = root.get("revision").map(text).transpose()?;
            Ok(Source {
                format: format.clone(),
                sha256: sha256.into(),
                api: text(&root["name"])?,
                version: text(&root["version"])?,
                source_uri: None,
                reference_sources: Default::default(),
                specification_version: "v1".into(),
                revision,
            })
        }
    }
}
