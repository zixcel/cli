//! Test-only executable. Release packaging selects --bin zix and excludes this probe.
use serde_json::{Value, json};
use std::{
    io::{self, Read, Write},
    time::Duration,
};
fn main() {
    assert_eq!(
        std::env::args().skip(1).collect::<Vec<_>>(),
        ["--zix-provider-v1"]
    );
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let request: Value = serde_json::from_str(&input).unwrap();
    let mode = request["arguments"]["mode"].as_str().unwrap_or("echo");
    if mode.starts_with("write-then-") {
        let path = request["arguments"]["value"].as_str().unwrap();
        let mut journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(journal, "synthetic_write").unwrap();
        match mode {
            "write-then-timeout" => std::thread::sleep(Duration::from_secs(60)),
            "write-then-invalid" => println!("invalid response after mutation"),
            "write-then-exit" => std::process::exit(9),
            _ => panic!("unsupported test scenario"),
        }
        return;
    }
    if mode == "hang-tree" {
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("60")
            .spawn()
            .unwrap();
        let path = request["arguments"]["value"].as_str().unwrap();
        std::fs::write(
            path,
            json!({"provider":std::process::id(),"child":child.id()}).to_string(),
        )
        .unwrap();
        child.wait().unwrap();
        return;
    }
    if mode == "hang" {
        std::thread::sleep(Duration::from_secs(60));
        return;
    }
    if mode == "stdout-limit" {
        print!("{}", "x".repeat(131073));
        return;
    }
    if mode == "stderr-limit" {
        io::stderr().write_all(&vec![b'x'; 8193]).unwrap();
        return;
    }
    if mode == "exit" {
        std::process::exit(9);
    }
    let mut result = json!({"value":request["arguments"].get("value"),
        "unselected_environment":std::env::var_os("LEAK_PROBE").is_some(),
        "selected_environment":std::env::var_os("ZIX_TEST_ACCESS_TOKEN").is_some(),
        "session_bus":std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()});
    if mode == "secret" {
        result = json!({"secret":std::env::var("ZIX_TEST_ACCESS_TOKEN").unwrap()});
    }
    let mut response = json!({"schema":"zix-provider/v1","id":request["id"],"operation":request["operation"],"result":result});
    if mode == "wrong-id" {
        response["id"] = json!("wrong");
    }
    if mode == "wrong-operation" {
        response["operation"] = json!("wrong");
    }
    if mode == "extra-field" {
        response["extra"] = json!(true);
    }
    if mode == "provider-error" {
        response.as_object_mut().unwrap().remove("result");
        response["error"] = json!("forbidden");
    }
    println!("{response}");
}
