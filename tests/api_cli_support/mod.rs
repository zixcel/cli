use super::{api_support::*, support::Context};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use zix_cli::files;
pub fn write(c: &Context, name: &str, value: &Value) -> PathBuf {
    let path = c.root.path().join(name);
    fs::write(&path, bytes(value)).unwrap();
    path
}
pub fn bound(c: &Context) -> PathBuf {
    let mut catalog = serde_json::to_value(compile(&source())).unwrap();
    catalog["operations"][0]["binding"] = json!({"namespace":"sample","command_id":"sample.inspect",
        "manifest_sha256":files::digest(&fs::read(&c.manifest).unwrap())});
    write(c, "catalog.json", &catalog)
}
