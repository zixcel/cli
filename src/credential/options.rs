use crate::{Error, Result, files};
use std::collections::BTreeMap;

#[cfg_attr(not(feature = "credential-store"), allow(dead_code))]
pub(crate) struct Request {
    pub action: String,
    pub collection: String,
    pub reference: String,
    pub host: String,
    pub environment: String,
    pub manifest_sha256: String,
    pub executable_sha256: String,
    pub command: Vec<String>,
    pub(super) bindings: Vec<super::bindings::Binding>,
}

pub(super) fn parse(args: &[String]) -> Result<Request> {
    let action = args.first().ok_or(Error("invalid_arguments"))?.as_str();
    let allowed: &[&str] = match action {
        "status" | "unlock" | "lock" => &["--collection"],
        "inspect" | "remove" => &["--collection", "--reference"],
        "set" => &["--collection", "--reference", "--host"],
        "run" if args.iter().any(|a| a == "--bindings") => &[
            "--collection",
            "--bindings",
            "--manifest-sha256",
            "--executable-sha256",
        ],
        "run" => &[
            "--collection",
            "--reference",
            "--host",
            "--environment",
            "--manifest-sha256",
            "--executable-sha256",
        ],
        _ => return Err(Error("invalid_arguments")),
    };
    let end = args.iter().position(|a| a == "--").unwrap_or(args.len());
    if end != 1 + allowed.len() * 2
        || (action == "run" && end + 2 >= args.len())
        || (action != "run" && end != args.len())
    {
        return Err(Error("invalid_arguments"));
    }
    let mut flags = BTreeMap::new();
    for pair in args[1..end].chunks_exact(2) {
        if !allowed.contains(&pair[0].as_str())
            || pair[1].is_empty()
            || flags.insert(pair[0].as_str(), pair[1].clone()).is_some()
        {
            return Err(Error("invalid_arguments"));
        }
    }
    let bindings = flags
        .remove("--bindings")
        .map(|p| super::bindings::read(&p))
        .transpose()?
        .unwrap_or_default();
    let mut get = |key| flags.remove(key).unwrap_or_default();
    let request = Request {
        action: action.into(),
        collection: get("--collection"),
        reference: get("--reference"),
        host: get("--host"),
        environment: get("--environment"),
        manifest_sha256: get("--manifest-sha256"),
        executable_sha256: get("--executable-sha256"),
        bindings,
        command: if end < args.len() {
            args[end + 1..].to_vec()
        } else {
            vec![]
        },
    };
    if request.collection == "session" || !component(&request.collection, 64) {
        return Err(Error("invalid_credential_collection"));
    }
    if allowed.contains(&"--reference") {
        let parts: Vec<_> = request.reference.split('/').collect();
        if parts.len() != 5 || parts.iter().any(|s| !component(s, 64)) {
            return Err(Error("invalid_credential_reference"));
        }
    }
    if allowed.contains(&"--host") && !valid_host(&request.host) {
        return Err(Error("invalid_credential_host"));
    }
    if action == "run"
        && (!files::valid_digest(&request.manifest_sha256)
            || !files::valid_digest(&request.executable_sha256)
            || (request.bindings.is_empty()
                && (!request.environment.starts_with("ZIX_")
                    || request.environment.len() > 80
                    || !request
                        .environment
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'))))
    {
        return Err(Error("invalid_credential_binding"));
    }
    Ok(request)
}
fn component(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}
fn valid_host(host: &str) -> bool {
    host.len() <= 253
        && host.contains('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

pub(super) fn validate_binding(b: &super::bindings::Binding) -> Result<()> {
    let parts: Vec<_> = b.reference.split('/').collect();
    if parts.len() != 5
        || parts.iter().any(|p| !component(p, 64))
        || !valid_host(&b.host)
        || !b.environment.starts_with("ZIX_")
        || b.environment.len() > 80
        || !b
            .environment
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        || b.field.as_ref().is_some_and(|f| !component(f, 64))
    {
        return Err(Error("invalid_credential_binding"));
    }
    Ok(())
}
impl Request {
    pub(super) fn credentials(&self) -> Vec<super::bindings::Binding> {
        if self.bindings.is_empty() {
            vec![super::bindings::Binding {
                reference: self.reference.clone(),
                host: self.host.clone(),
                environment: self.environment.clone(),
                field: None,
            }]
        } else {
            self.bindings.clone()
        }
    }
}
