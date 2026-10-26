mod support;
use support::Context;

#[test]
fn extensions_cannot_shadow_the_credential_command() {
    let c = Context::new(1000);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
    manifest["namespace"] = serde_json::json!("credential");
    std::fs::write(&c.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let output = c.install();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid_manifest"));
}

#[test]
fn help_is_available_without_a_store_and_never_creates_state() {
    let c = Context::new(1000);
    let output = c.run(&["credential", "--help"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("no secret argv"));
    assert!(text.contains("--manifest-sha256"));
    assert!(!c.root.path().join("zix").exists());
}

#[test]
fn malformed_or_unsafe_requests_are_rejected_before_store_access() {
    let c = Context::new(1000);
    for args in [
        vec!["credential", "get", "--collection", "crowsi"],
        vec!["credential", "status", "--collection", "session"],
        vec!["credential", "status", "--collection", "../login"],
        vec![
            "credential",
            "status",
            "--collection",
            "crowsi",
            "--collection",
            "login",
        ],
        vec![
            "credential",
            "inspect",
            "--collection",
            "crowsi",
            "--reference",
            "local/provider/api/ns",
        ],
        vec![
            "credential",
            "set",
            "--collection",
            "crowsi",
            "--reference",
            "local/provider/api/ns/main",
            "--host",
            "https://api.example.com",
        ],
        vec![
            "credential",
            "set",
            "--collection",
            "crowsi",
            "--reference",
            "local/provider/api/ns/main",
            "--host",
            "api.example.com",
            "--secret",
            "forbidden",
        ],
    ] {
        let output = c.run(&args);
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("forbidden"));
        assert!(!c.root.path().join("zix").exists());
    }
}

#[cfg(feature = "credential-store")]
#[test]
fn enrollment_and_unlock_require_a_real_owner_terminal() {
    let c = Context::new(1000);
    for args in [
        vec!["credential", "unlock", "--collection", "crowsi"],
        vec![
            "credential",
            "set",
            "--collection",
            "crowsi",
            "--reference",
            "local/provider/api/sample/main",
            "--host",
            "api.example.com",
        ],
    ] {
        let output = c.run(&args);
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("interactive_authorization_required")
        );
    }
}

#[cfg(feature = "credential-store")]
#[test]
fn remote_session_bus_is_rejected() {
    let c = Context::new(1000);
    let output = c
        .command()
        .env("DBUS_SESSION_BUS_ADDRESS", "tcp:host=example.com,port=1234")
        .args(["credential", "status", "--collection", "default"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid_runtime_environment"));
}

#[cfg(feature = "credential-store")]
#[test]
fn credential_use_does_not_bypass_write_authorization() {
    let c = Context::new(1000);
    let mut m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
    m["schema"] = serde_json::json!("zix-extension/v2");
    m["commands"][0]["effect"] = serde_json::json!("operator-write");
    m["commands"][0]["arguments"] = serde_json::json!([
        {"name":"plan-file","required":true,"maximum":2048},
        {"name":"approval-sha256","required":true,"maximum":64},
        {"name":"journal-directory","required":true,"maximum":2048}]);
    std::fs::write(&c.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(c.install().status.success());
    let digest = |p: &std::path::Path| zix_cli::files::digest(&std::fs::read(p).unwrap());
    let output = c.run(&[
        "credential",
        "run",
        "--collection",
        "default",
        "--reference",
        "local/provider/api/sample/main",
        "--host",
        "api.example.com",
        "--environment",
        "ZIX_TEST_ACCESS_TOKEN",
        "--manifest-sha256",
        &digest(&c.manifest),
        "--executable-sha256",
        &digest(&c.executable),
        "--",
        "sample",
        "inspect",
        "--plan-file",
        "/nonexistent/plan.json",
        "--approval-sha256",
        &"a".repeat(64),
        "--journal-directory",
        "/nonexistent/journal",
    ]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("interactive_authorization_required"));
}

#[cfg(not(feature = "credential-store"))]
#[test]
fn static_build_refuses_instead_of_falling_back_to_plaintext() {
    let c = Context::new(1000);
    let output = c.run(&["credential", "status", "--collection", "crowsi"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("credential_store_not_in_build"));
}

#[cfg(feature = "credential-store")]
#[test]
fn target_hash_audience_and_environment_are_checked_before_store_access() {
    let c = Context::new(1000);
    assert!(c.install().status.success());
    let digest = |p: &std::path::Path| zix_cli::files::digest(&std::fs::read(p).unwrap());
    let manifest = digest(&c.manifest);
    let executable = digest(&c.executable);
    let zero = "0".repeat(64);
    let base = |reference: &str, environment: &str, m: &str, e: &str| {
        vec![
            "credential".to_string(),
            "run".into(),
            "--collection".into(),
            "crowsi".into(),
            "--reference".into(),
            reference.into(),
            "--host".into(),
            "api.example.com".into(),
            "--environment".into(),
            environment.into(),
            "--manifest-sha256".into(),
            m.into(),
            "--executable-sha256".into(),
            e.into(),
            "--".into(),
            "sample".into(),
            "inspect".into(),
            "--mode".into(),
            "normal".into(),
        ]
    };
    for args in [
        base(
            "local/provider/api/sample/main",
            "ZIX_TEST_ACCESS_TOKEN",
            &zero,
            &executable,
        ),
        base(
            "local/provider/api/sample/main",
            "ZIX_TEST_ACCESS_TOKEN",
            &manifest,
            &zero,
        ),
        base(
            "local/provider/api/other/main",
            "ZIX_TEST_ACCESS_TOKEN",
            &manifest,
            &executable,
        ),
        base(
            "local/provider/api/sample/main",
            "ZIX_OTHER_TOKEN",
            &manifest,
            &executable,
        ),
    ] {
        let output = c.command().args(args).output().unwrap();
        assert!(String::from_utf8_lossy(&output.stderr).contains("credential_target_mismatch"));
    }
    let args = base(
        "local/provider/api/sample/main",
        "ZIX_TEST_ACCESS_TOKEN",
        &manifest,
        &executable,
    );
    let output = c
        .command()
        .env("ZIX_TEST_ACCESS_TOKEN", "synthetic")
        .args(&args)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("credential_environment_conflict"));
    assert!(c.run(&["extensions", "disable", "sample"]).status.success());
    let output = c.command().args(&args).output().unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("disabled"));
}
