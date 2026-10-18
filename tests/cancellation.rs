mod support;
use serde_json::Value;
use std::{
    fs,
    process::Stdio,
    time::{Duration, Instant},
};
use support::Context;
#[test]
fn interrupt_stops_the_owned_provider_and_its_descendant() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    let state = c.root.path().join("probe-state.json");
    let child = c
        .command()
        .args([
            "sample",
            "inspect",
            "--mode",
            "hang-tree",
            "--value",
            state.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !state.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(state.exists());
    let ids: Value = serde_json::from_slice(&fs::read(state).unwrap()).unwrap();
    rustix::process::kill_process(
        rustix::process::Pid::from_raw(child.id() as i32).unwrap(),
        rustix::process::Signal::INT,
    )
    .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("cancelled"));
    for key in ["provider", "child"] {
        let path = format!("/proc/{}/stat", ids[key].as_u64().unwrap());
        if let Ok(stat) = fs::read_to_string(path) {
            let rest = stat.rsplit_once(") ").unwrap().1;
            assert!(
                rest.starts_with('Z') || rest.starts_with('X'),
                "owned process still running"
            );
        }
    }
}
