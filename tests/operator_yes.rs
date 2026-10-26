mod support;
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use support::Context;
use zix_operation_contracts::{Binding, LocalPlan};
fn fixture() -> (Context, PathBuf, LocalPlan) {
    let c = Context::new(1000);
    let mut m: Value = serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
    m["schema"] = json!("zix-extension/v2");
    m["commands"][0]["effect"] = json!("operator-write");
    m["commands"][0]["arguments"] = json!([
 {"name":"plan-file","required":true,"maximum":2048},
 {"name":"approval-sha256","required":true,"maximum":64},
 {"name":"journal-directory","required":true,"maximum":2048},
 {"name":"mode","required":false,"maximum":32}]);
    std::fs::write(&c.manifest, m.to_string()).unwrap();
    assert!(c.install().status.success());
    let p = LocalPlan::new(
        "sample.inspect".into(),
        Binding::for_catalog(
            "sample-provider",
            "0.1.0",
            &m,
            &zix_cli::files::digest(&std::fs::read(&c.executable).unwrap()),
        ),
        json!({"synthetic":true}),
    );
    let path = c.root.path().join("plan.json");
    store(&path, &p);
    (c, path, p)
}
fn store(path: &std::path::Path, p: &LocalPlan) {
    std::fs::write(path, serde_json::to_vec(p).unwrap()).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}
#[test]
fn yes_aliases_work_without_terminal_or_manual_digest_and_are_not_forwarded() {
    let (c, path, p) = fixture();
    for flag in ["-y", "--yes"] {
        for leading in [false, true] {
            let mut args = vec![
                "sample",
                "inspect",
                "--plan-file",
                path.to_str().unwrap(),
                "--journal-directory",
                c.root.path().to_str().unwrap(),
                "--mode",
                "consent-inspect",
            ];
            if leading {
                args.insert(0, flag)
            } else {
                args.push(flag)
            }
            let out = c
                .command()
                .env("GITHUB_ACTIONS", "true")
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let v: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(v["result"]["approval_sha256"], p.digest());
            assert_eq!(v["result"]["confirmation_forwarded"], false);
        }
    }
}
#[test]
fn yes_does_not_bypass_pinned_hash_operation_binding_or_private_file_checks() {
    let (c, path, p) = fixture();
    let base = vec![
        "sample",
        "inspect",
        "--plan-file",
        path.to_str().unwrap(),
        "--journal-directory",
        c.root.path().to_str().unwrap(),
        "-y",
    ];
    let mut args = base.clone();
    let wrong = "0".repeat(64);
    args.extend(["--approval-sha256", &wrong]);
    let r = c.run(&args);
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("plan_binding_mismatch"));
    for operation in ["wrong.operation", "sample.inspect"] {
        let mut bad = p.clone();
        bad.operation = operation.into();
        if operation == "sample.inspect" {
            bad.binding.version = "0.0.0".into();
        }
        store(&path, &bad);
        let r = c.run(&base);
        assert!(!r.status.success());
        assert!(String::from_utf8_lossy(&r.stderr).contains("plan_binding_mismatch"));
    }
    store(&path, &p);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let r = c.run(&base);
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("unsafe_plan_file"));
}
