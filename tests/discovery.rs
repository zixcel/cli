mod support;
use serde_json::Value;
use support::Context;
#[test]
fn discovery_is_deterministic_and_reflects_enablement() {
    let c = Context::new(5000);
    assert!(c.install().status.success());
    for args in [
        vec!["list"],
        vec!["capabilities", "--format", "json"],
        vec!["describe", "sample", "--format", "json"],
        vec!["prompt", "sample"],
    ] {
        let first = c.run(&args);
        assert!(first.status.success());
        assert_eq!(first.stdout, c.run(&args).stdout);
    }
    let prompt = String::from_utf8(c.run(&["prompt", "sample"]).stdout).unwrap();
    assert!(prompt.contains(" --mode '<value>'"));
    assert!(prompt.contains(" [--value '<value>']"));
    assert!(!prompt.contains("[--mode"));
    assert!(c.run(&["extensions", "disable", "sample"]).status.success());
    let out = c.run(&["list", "--format", "json"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["providers"],
        serde_json::json!([])
    );
}
