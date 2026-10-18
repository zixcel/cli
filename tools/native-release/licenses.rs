use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn notice(name: &str) -> bool {
    [
        "LICENSE",
        "LICENCE",
        "COPYING",
        "COPYRIGHT",
        "NOTICE",
        "AUTHORS",
    ]
    .iter()
    .any(|prefix| name.to_ascii_uppercase().starts_with(prefix))
}
pub fn collect(output: &Path) -> Result<(), &'static str> {
    let cargo = std::env::var_os("CARGO").ok_or("cargo_missing")?;
    let result = Command::new(cargo)
        .args(["metadata", "--locked", "--format-version=1"])
        .output()
        .map_err(|_| "metadata_failed")?;
    if !result.status.success() || result.stdout.len() > 16 * 1024 * 1024 {
        return Err("metadata_failed");
    }
    let meta: Value = serde_json::from_slice(&result.stdout).map_err(|_| "metadata_failed")?;
    let mut inventory = Vec::new();
    for package in meta["packages"].as_array().ok_or("metadata_failed")? {
        let name = package["name"].as_str().ok_or("metadata_failed")?;
        let version = package["version"].as_str().ok_or("metadata_failed")?;
        if [name, version].iter().any(|s| {
            s.len() > 100
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
        }) {
            return Err("package_name_rejected");
        }
        inventory.push(json!({"name":name,"version":version,"license":package["license"],"source":package["source"]}));
        if package["source"].is_null() {
            continue;
        }
        let original = Path::new(package["manifest_path"].as_str().ok_or("metadata_failed")?)
            .parent()
            .ok_or("metadata_failed")?;
        let retained = Path::new("tools/native-release/notices").join(format!("{name}-{version}"));
        let root = if find(original, 0)?.is_empty() && retained.is_dir() {
            retained.as_path()
        } else {
            original
        };
        let files = find(root, 0)?;
        if files.is_empty() {
            eprintln!("missing license material: {name}@{version}");
            return Err("dependency_notice_missing");
        }
        let destination = output.join("THIRD-PARTY").join(format!("{name}-{version}"));
        for file in files {
            let bytes = fs::read(&file).map_err(|_| "notice_unavailable")?;
            if bytes.len() > 1024 * 1024
                || bytes.contains(&0)
                || std::str::from_utf8(&bytes).is_err()
            {
                return Err("notice_rejected");
            }
            let relative = file.strip_prefix(root).map_err(|_| "notice_rejected")?;
            let target = destination.join(relative);
            fs::create_dir_all(target.parent().ok_or("notice_rejected")?)
                .map_err(|_| "output_unavailable")?;
            fs::write(target, bytes).map_err(|_| "output_unavailable")?;
        }
    }
    inventory.sort_by(|a, b| {
        a["name"]
            .as_str()
            .cmp(&b["name"].as_str())
            .then(a["version"].as_str().cmp(&b["version"].as_str()))
    });
    let record = json!({"schema":"zix-build-dependency-notices/v1","coverage":"resolved build and development graph; not a linked-binary SBOM","packages":inventory});
    fs::write(
        output.join("DEPENDENCIES.json"),
        serde_json::to_vec_pretty(&record).map_err(|_| "metadata_failed")?,
    )
    .map_err(|_| "output_unavailable")?;
    Ok(())
}
fn find(path: &Path, depth: u8) -> Result<Vec<PathBuf>, &'static str> {
    if depth > 3 {
        return Ok(vec![]);
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(path).map_err(|_| "notice_unavailable")? {
        let entry = entry.map_err(|_| "notice_unavailable")?;
        let kind = entry.file_type().map_err(|_| "notice_unavailable")?;
        if kind.is_symlink() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "notice_rejected")?;
        if kind.is_file() && (depth > 0 || notice(&name)) {
            files.push(entry.path());
        } else if kind.is_dir()
            && (depth > 0
                || ["LICENSES", "LICENCES", "LICENSE", "LICENCE"]
                    .contains(&name.to_ascii_uppercase().as_str()))
        {
            files.extend(find(&entry.path(), depth + 1)?);
        }
        if files.len() > 256 {
            return Err("too_many_notices");
        }
    }
    files.sort();
    Ok(files)
}
