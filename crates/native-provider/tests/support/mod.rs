use serde_json::{Value, json};
use zix_native_provider::{Provider, Request};
pub const ACCOUNT: &str = "00000000-0000-0000-0000-000000000001";
pub fn documents() -> (Value, Value) {
    let routes = json!({"schema":"zix-api-routes/v1","source":{"format":"openapi","specification_version":"3.1.0",
        "sha256":"0".repeat(64),"api":"Synthetic","version":"1","revision":null},
        "operations":[{"id":"service.items.get","method":"GET","path":"/accounts/{account}/items/{item}","servers":["https://service.example.invalid/api"]}]});
    let policy = json!({"schema":"zix-native-policy/v1","namespace":"sample","package":"synthetic-native-provider","version":"0.1.0",
        "credential_environment":"ZIX_SAMPLE_ACCESS_TOKEN","origin":"https://service.example.invalid","base_path":"/api",
        "routes_sha256":zix_cli::files::digest(&serde_json::to_vec(&routes).unwrap()),
        "operations":[{"api":"service.items.get","command":{"id":"item.read","words":["items","get"],"effect":"read",
        "description":"Observe one selected synthetic item.","required_permissions":["items.read"],
        "arguments":[{"name":"account","required":true,"maximum":36},{"name":"item","required":true,"maximum":128}]},
        "path_arguments":{"account":"account","item":"item"},"argument_rules":{"account":"uuid","item":"identifier"},
        "query":{"limit":"20"},"headers":{},"identity":[{"pointer":"/account","expected":"{account}"},{"pointer":"/id","expected":"{item}"}],
        "projection":{"pointer":"","collection":false,"fields":{"id":"/id","name":"/name"},"required":["id"],"success_pointer":null}}]});
    (routes, policy)
}
pub fn load(routes: &Value, policy: &Value) -> Result<Provider, zix_native_provider::Error> {
    Provider::load(
        &serde_json::to_vec(routes).unwrap(),
        &serde_json::to_vec(policy).unwrap(),
        "0.1.0",
    )
}
pub fn provider() -> Provider {
    let (r, p) = documents();
    load(&r, &p).unwrap()
}
pub fn request() -> Request {
    serde_json::from_value(json!({"schema":"zix-provider/v1","id":"0".repeat(32),"operation":"item.read",
        "arguments":{"account":ACCOUNT,"item":"synthetic-item"},"limits":{"timeout_ms":5000,"maximum_response_bytes":65536}})).unwrap()
}
