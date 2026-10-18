use super::{archive, licenses};
use crate::{files, manifest::Manifest};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
pub struct Identity<'a> {
    pub package: &'a str,
    pub version: &'a str,
    pub repository: &'a str,
    pub source: &'a str,
}
pub fn bundle(
    identity: Identity<'_>,
    binary: &Path,
    output: &Path,
    target: &str,
) -> Result<(), &'static str> {
    if !["x86_64-unknown-linux-musl", "x86_64-unknown-linux-gnu"].contains(&target)
        || identity.source.len() != 40
        || !identity
            .source
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || identity.package.is_empty()
        || identity.package.len() > 100
        || !identity
            .package
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        || identity.version.split('.').count() != 3
        || identity
            .version
            .split('.')
            .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
        || !identity.repository.starts_with("https://github.com/")
    {
        return Err("release_identity_rejected");
    }
    let filename = binary
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("binary_rejected")?;
    if !filename.starts_with("zix-")
        || filename.len() > 80
        || !filename
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("binary_rejected");
    }
    let bytes = files::read(binary, 64 * 1024 * 1024, true).map_err(|_| "binary_rejected")?;
    if !bytes.starts_with(b"\x7fELF") {
        return Err("binary_rejected");
    }
    let name = format!("{}-v{}-{target}", identity.package, identity.version);
    fs::create_dir_all(output).map_err(|_| "output_unavailable")?;
    let directory = output.join(&name);
    fs::create_dir(&directory).map_err(|_| "output_exists")?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
        .map_err(|_| "output_unavailable")?;
    let copied = directory.join(filename);
    fs::write(&copied, &bytes).map_err(|_| "output_unavailable")?;
    fs::set_permissions(&copied, fs::Permissions::from_mode(0o500))
        .map_err(|_| "output_unavailable")?;
    let result = Command::new(&copied)
        .arg("--manifest")
        .env_clear()
        .output()
        .map_err(|_| "manifest_failed")?;
    if !result.status.success() {
        return Err("manifest_failed");
    }
    let manifest = Manifest::parse(&result.stdout).map_err(|_| "manifest_failed")?;
    if manifest.package != identity.package || manifest.version != identity.version {
        return Err("manifest_identity_mismatch");
    }
    fs::write(
        directory.join(format!("{}.extension.json", manifest.namespace)),
        &result.stdout,
    )
    .map_err(|_| "output_unavailable")?;
    for file in ["LICENSE", "NOTICE"] {
        fs::copy(file, directory.join(file)).map_err(|_| "owner_notice_missing")?;
    }
    licenses::collect(&directory)?;
    let record = serde_json::json!({"schema":"zix-native-release/v1","package":identity.package,"version":identity.version,
        "repository":identity.repository,"source":identity.source,"target":target,"binary":filename,
        "namespace":manifest.namespace,"binary_sha256":files::digest(&bytes),"manifest_sha256":files::digest(&result.stdout)});
    fs::write(
        directory.join("RELEASE.json"),
        serde_json::to_vec_pretty(&record).map_err(|_| "invalid_record")?,
    )
    .map_err(|_| "output_unavailable")?;
    archive::finish(output, &directory, &name)
}
