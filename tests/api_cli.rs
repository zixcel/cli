mod api_cli_support;
mod api_support;
mod support;
use api_cli_support::*;
use api_support::*;
use serde_json::Value;
use std::fs;
use support::Context;
use zix_cli::files;
#[test]
fn specification_import_and_describe_work_offline_without_a_registry_or_home() {
    let c = Context::new(5000);
    let source = bytes(&source());
    let spec = c.root.path().join("source.json");
    fs::write(&spec, &source).unwrap();
    let review = write(
        &c,
        "review.json",
        &review(&source, "openapi", "sample.inspect"),
    );
    let out = c
        .command()
        .env_remove("XDG_CONFIG_HOME")
        .args([
            "api",
            "import",
            spec.to_str().unwrap(),
            review.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout.iter().filter(|b| **b == b'\n').count(), 1);
    let catalog = c.root.path().join("imported.json");
    fs::write(&catalog, &out.stdout).unwrap();
    let described = c.run(&["api", "describe", catalog.to_str().unwrap()]);
    assert!(described.status.success());
    assert_eq!(described.stdout, out.stdout);
    assert!(
        c.run(&[
            "api",
            "diff",
            catalog.to_str().unwrap(),
            catalog.to_str().unwrap()
        ])
        .status
        .success()
    );
}
#[test]
fn reviewed_operation_dispatches_through_registered_provider_without_a_shell() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let path = bound(&c);
    let digest = files::digest(&fs::read(&path).unwrap());
    let out = c
        .command()
        .env("LEAK_PROBE", "unselected")
        .args([
            "api",
            "run",
            path.to_str().unwrap(),
            &digest,
            "sample.inspect",
            "--mode",
            "echo",
            "--value",
            "$(exit 4); literal",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let response: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(response["result"]["value"], "$(exit 4); literal");
    assert_eq!(response["result"]["unselected_environment"], false);
}
