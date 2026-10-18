mod support;
use serde_json::Value;
use support::Context;
#[test]
fn registered_absolute_provider_executes_through_crowsi_with_shell_free_arguments() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let marker = c.root.path().join("must-not-execute");
    let value = format!("$(touch '{}'); 'literal'", marker.display());
    let out = c
        .command()
        .env("LEAK_PROBE", "NOT_FOR_PROVIDER")
        .env("ZIX_TEST_ACCESS_TOKEN", "SYNTHETIC_ONLY")
        .args(["sample", "inspect", "--mode", "echo", "--value", &value])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["result"]["value"], value);
    assert_eq!(r["result"]["unselected_environment"], false);
    assert_eq!(r["result"]["selected_environment"], true);
    assert!(!marker.exists());
}
#[test]
fn missing_disabled_removed_and_changed_executable_never_fallback() {
    let c = Context::new(5000);
    assert!(
        !c.run(&["sample", "inspect", "--mode", "echo"])
            .status
            .success()
    );
    assert!(c.install().status.success());
    assert!(c.run(&["extensions", "disable", "sample"]).status.success());
    assert!(
        !c.run(&["sample", "inspect", "--mode", "echo"])
            .status
            .success()
    );
    assert!(c.run(&["extensions", "enable", "sample"]).status.success());
    std::fs::write(&c.executable, b"changed").unwrap();
    let out = c.run(&["sample", "inspect", "--mode", "echo"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("integrity_mismatch"));
    assert!(c.run(&["extensions", "remove", "sample"]).status.success());
    assert!(
        !c.run(&["sample", "inspect", "--mode", "echo"])
            .status
            .success()
    );
}
#[test]
fn wrong_responses_exit_codes_and_both_pipe_limits_fail_closed() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    for mode in [
        "wrong-id",
        "wrong-operation",
        "extra-field",
        "exit",
        "stdout-limit",
        "stderr-limit",
    ] {
        let out = c.run(&["sample", "inspect", "--mode", mode]);
        assert!(!out.status.success(), "{mode}");
        assert!(out.stdout.is_empty());
    }
}
#[test]
fn selected_credential_cannot_be_echoed_and_provider_errors_are_not_success() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let secret = "SYNTHETIC_ACCESS_TOKEN_\"_ONLY";
    let out = c
        .command()
        .env("ZIX_TEST_ACCESS_TOKEN", secret)
        .args(["sample", "inspect", "--mode", "secret"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&out.stderr).contains(secret));
    let out = c.run(&["sample", "inspect", "--mode", "provider-error"]);
    assert!(!out.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["error"],
        "forbidden"
    );
}
#[test]
fn child_timeout_is_bounded_without_replaying_the_request() {
    let c = Context::new(100);
    assert!(c.install().status.success());
    let start = std::time::Instant::now();
    let out = c.run(&["sample", "inspect", "--mode", "hang"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("provider_timeout"));
    assert!(start.elapsed() < std::time::Duration::from_secs(4));
}
