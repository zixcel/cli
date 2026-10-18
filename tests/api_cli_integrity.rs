mod api_cli_support;
mod api_support;
mod support;
use api_cli_support::*;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::symlink};
use support::Context;
use zix_cli::files;
#[test]
fn catalog_tampering_unbound_methods_provider_drift_and_unknown_arguments_refuse_execution() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let path = bound(&c);
    let mut catalog: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let digest = files::digest(&fs::read(&path).unwrap());
    let output = c.run(&[
        "api",
        "run",
        path.to_str().unwrap(),
        &"0".repeat(64),
        "sample.inspect",
    ]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("api_catalog_integrity_mismatch"));
    assert!(
        !c.run(&[
            "api",
            "run",
            path.to_str().unwrap(),
            &digest,
            "sample.inspect",
            "--unknown",
            "echo"
        ])
        .status
        .success()
    );
    assert!(
        !c.run(&[
            "api",
            "run",
            path.to_str().unwrap(),
            &digest,
            "unreviewed.delete"
        ])
        .status
        .success()
    );
    catalog["operations"][0]["binding"]["manifest_sha256"] = json!("0".repeat(64));
    let drift = write(&c, "drift.json", &catalog);
    let output = c.run(&[
        "api",
        "run",
        drift.to_str().unwrap(),
        &files::digest(&fs::read(&drift).unwrap()),
        "sample.inspect",
    ]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("api_provider_drift"));
    catalog["operations"][0]["binding"] = Value::Null;
    let unbound = write(&c, "unbound.json", &catalog);
    let output = c.run(&[
        "api",
        "run",
        unbound.to_str().unwrap(),
        &files::digest(&fs::read(&unbound).unwrap()),
        "sample.inspect",
    ]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("api_operation_not_bound"));
    assert_eq!(
        c.run(&[
            "api",
            "diff",
            path.to_str().unwrap(),
            unbound.to_str().unwrap()
        ])
        .status
        .code(),
        Some(2)
    );
    let link = c.root.path().join("catalog-link.json");
    symlink(&path, &link).unwrap();
    assert!(
        !c.run(&["api", "describe", link.to_str().unwrap()])
            .status
            .success()
    );
}
