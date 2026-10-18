use std::{
    fs,
    path::{Path, PathBuf},
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
pub(super) fn find(path: &Path, depth: u8) -> Result<Vec<PathBuf>, &'static str> {
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
