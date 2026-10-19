mod support;
use serde_json::{Value, json};
use support::Context;
fn candidate(c: &Context, schema: &str) -> Value {
    let mut m: Value = serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
    m["schema"] = json!(schema);
    m["commands"][0]["effect"] = json!("operator-write");
    m["commands"][0]["arguments"] = json!([
 {"name":"plan-file","required":true,"maximum":2048},
 {"name":"approval-sha256","required":true,"maximum":64},
 {"name":"journal-directory","required":true,"maximum":2048}]);
    m
}
#[test]
fn legacy_read_only_schema_cannot_smuggle_a_mutation() {
    let c = Context::new(1000);
    let m = candidate(&c, "zix-extension/v1");
    std::fs::write(&c.manifest, m.to_string()).unwrap();
    assert!(!c.install().status.success());
}
#[test]
fn write_manifest_needs_explicit_plan_hash_and_journal_and_unattended_execution_never_launches() {
    let c = Context::new(1000);
    let m = candidate(&c, "zix-extension/v2");
    std::fs::write(&c.manifest, m.to_string()).unwrap();
    assert!(c.install().status.success());
    let out = c.run(&[
        "sample",
        "inspect",
        "--plan-file",
        "/nonexistent/plan.json",
        "--approval-sha256",
        &"a".repeat(64),
        "--journal-directory",
        "/nonexistent/journal",
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("interactive_authorization_required"));
    let mut invalid = m;
    invalid["commands"][0]["arguments"] = json!([]);
    std::fs::write(&c.manifest, invalid.to_string()).unwrap();
    assert!(!c.install().status.success());
}
#[test]
fn explicit_terminal_confirmation_binds_the_actual_registered_plan_before_launch() {
    use std::{
        io::Write,
        os::unix::fs::PermissionsExt,
        process::{Command, Stdio},
    };
    use zix_operation_contracts::{Binding, LocalPlan};
    let c = Context::new(1000);
    let m = candidate(&c, "zix-extension/v2");
    std::fs::write(&c.manifest, m.to_string()).unwrap();
    assert!(c.install().status.success());
    let binding = Binding::for_catalog(
        "sample-provider",
        "0.1.0",
        &m,
        &zix_cli::files::digest(&std::fs::read(&c.executable).unwrap()),
    );
    let p = LocalPlan::new("sample.inspect".into(), binding, json!({"synthetic":true}));
    let path = c.root.path().join("plan.json");
    std::fs::write(&path, serde_json::to_vec(&p).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let quote = |v: &str| format!("'{}'", v.replace('\'', "'\\''"));
    let command = [
        env!("CARGO_BIN_EXE_zix"),
        "sample",
        "inspect",
        "--plan-file",
        path.to_str().unwrap(),
        "--approval-sha256",
        &p.digest(),
        "--journal-directory",
        c.root.path().to_str().unwrap(),
    ]
    .into_iter()
    .map(quote)
    .collect::<Vec<_>>()
    .join(" ");
    for accepted in [false, true] {
        let mut child = Command::new("script")
            .args(["--quiet", "--return", "--command", &command, "/dev/null"])
            .env_remove("GITHUB_ACTIONS")
            .env("XDG_CONFIG_HOME", c.root.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("util-linux script required for real terminal acceptance");
        writeln!(
            child.stdin.take().unwrap(),
            "{}",
            if accepted { p.digest() } else { String::new() }
        )
        .unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.success(),
            accepted,
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
        if accepted {
            assert!(String::from_utf8_lossy(&out.stdout).contains("unselected_environment"));
        }
    }
}
