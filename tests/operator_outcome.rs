mod support;
use serde_json::{Value, json};
use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};
use support::Context;
use zix_operation_contracts::{Binding, LocalPlan};

#[test]
fn interrupted_write_responses_require_readback_and_never_trigger_a_retry() {
    for mode in [
        "write-then-timeout",
        "write-then-invalid",
        "write-then-exit",
    ] {
        let c = Context::new(1000);
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
        manifest["schema"] = json!("zix-extension/v2");
        manifest["commands"][0]["effect"] = json!("operator-write");
        let arguments = manifest["commands"][0]["arguments"].as_array_mut().unwrap();
        for (name, maximum) in [
            ("plan-file", 2048),
            ("approval-sha256", 64),
            ("journal-directory", 2048),
        ] {
            arguments.push(json!({"name":name,"required":true,"maximum":maximum}));
        }
        std::fs::write(&c.manifest, manifest.to_string()).unwrap();
        assert!(c.install().status.success());
        let plan = LocalPlan::new(
            "sample.inspect".into(),
            Binding::for_catalog(
                "sample-provider",
                "0.1.0",
                &manifest,
                &zix_cli::files::digest(&std::fs::read(&c.executable).unwrap()),
            ),
            json!({"synthetic":true}),
        );
        let path = c.root.path().join("plan.json");
        std::fs::write(&path, serde_json::to_vec(&plan).unwrap()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let state = c.root.path().join("writes.txt");
        let quote = |text: &str| format!("'{}'", text.replace('\'', "'\\''"));
        let command = [
            env!("CARGO_BIN_EXE_zix"),
            "sample",
            "inspect",
            "--mode",
            mode,
            "--value",
            state.to_str().unwrap(),
            "--plan-file",
            path.to_str().unwrap(),
            "--approval-sha256",
            &plan.digest(),
            "--journal-directory",
            c.root.path().to_str().unwrap(),
        ]
        .into_iter()
        .map(quote)
        .collect::<Vec<_>>()
        .join(" ");
        let mut child = Command::new("script")
            .args(["--quiet", "--return", "--command", &command, "/dev/null"])
            .env_remove("GITHUB_ACTIONS")
            .env("XDG_CONFIG_HOME", c.root.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        writeln!(child.stdin.take().unwrap(), "yes").unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("operator_write_outcome_unknown"), "{text}");
        assert!(text.contains("provider_readback_required"), "{text}");
        assert_eq!(std::fs::read_to_string(state).unwrap(), "synthetic_write\n");
    }
}
