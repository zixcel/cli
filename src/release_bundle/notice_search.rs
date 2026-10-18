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

pub(super) fn select(original: &Path, retained: &[PathBuf]) -> Result<PathBuf, &'static str> {
    if !find(original, 0)?.is_empty() {
        return Ok(original.into());
    }
    for candidate in retained {
        if candidate.is_dir() && !find(candidate, 0)?.is_empty() {
            return Ok(candidate.clone());
        }
    }
    Ok(original.into()) // Collection still fails closed when no material is available.
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_notices_only_fill_missing_upstream_material() {
        let temporary = tempfile::tempdir().unwrap();
        let original = temporary.path().join("original");
        let owner = temporary.path().join("owner");
        fs::create_dir(&original).unwrap();
        fs::create_dir(&owner).unwrap();
        assert_eq!(
            select(&original, std::slice::from_ref(&owner)).unwrap(),
            original
        );
        fs::write(owner.join("LICENSE-MIT"), "retained original license").unwrap();
        assert_eq!(
            select(&original, std::slice::from_ref(&owner)).unwrap(),
            owner
        );
        fs::write(original.join("LICENSE-MIT"), "upstream package license").unwrap();
        assert_eq!(select(&original, &[owner]).unwrap(), original);
    }
}
