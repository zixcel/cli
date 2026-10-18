use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn files(root: &Path, path: &Path) -> Result<Vec<PathBuf>, &'static str> {
    let mut result = Vec::new();
    for e in fs::read_dir(path).map_err(|_| "output_unavailable")? {
        let e = e.map_err(|_| "output_unavailable")?;
        let kind = e.file_type().map_err(|_| "output_unavailable")?;
        if kind.is_dir() {
            result.extend(files(root, &e.path())?);
        } else if kind.is_file() {
            result.push(
                e.path()
                    .strip_prefix(root)
                    .map_err(|_| "output_unavailable")?
                    .to_owned(),
            );
        } else {
            return Err("nonregular_output");
        }
    }
    result.sort();
    Ok(result)
}
pub fn finish(output: &Path, directory: &Path, name: &str) -> Result<(), &'static str> {
    let mut checks = String::new();
    for path in files(directory, directory)? {
        let bytes = fs::read(directory.join(&path)).map_err(|_| "output_unavailable")?;
        checks.push_str(&format!(
            "{}  {}\n",
            hash(&bytes),
            path.to_str().ok_or("output_unavailable")?
        ));
    }
    fs::write(directory.join("CONTENTS.sha256"), checks).map_err(|_| "output_unavailable")?;
    let filename = format!("{name}.tar.gz");
    let destination = output.join(&filename);
    let archive = fs::File::create(&destination).map_err(|_| "output_unavailable")?;
    let mut tar = Command::new("tar")
        .args([
            "--sort=name",
            "--mtime=@0",
            "--owner=0",
            "--group=0",
            "--numeric-owner",
            "-cf",
            "-",
        ])
        .arg("-C")
        .arg(output)
        .arg(name)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| "archive_failed")?;
    let gzip = Command::new("gzip")
        .arg("-n")
        .stdin(tar.stdout.take().ok_or("archive_failed")?)
        .stdout(Stdio::from(archive))
        .status()
        .map_err(|_| "archive_failed")?;
    if !tar.wait().map_err(|_| "archive_failed")?.success() || !gzip.success() {
        return Err("archive_failed");
    }
    let bytes = fs::read(destination).map_err(|_| "output_unavailable")?;
    fs::write(
        output.join("SHA256SUMS"),
        format!("{}  {filename}\n", hash(&bytes)),
    )
    .map_err(|_| "output_unavailable")?;
    Ok(())
}
