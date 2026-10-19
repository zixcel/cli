#![forbid(unsafe_code)]
//! Integrity bindings for explicit operator-reviewed local plans; not signatures or an authority.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub package: String,
    pub version: String,
    pub catalog_sha256: String,
    pub executable_sha256: String,
}
impl Binding {
    pub fn for_catalog(
        package: &str,
        version: &str,
        catalog: &Value,
        executable_sha256: &str,
    ) -> Self {
        Self {
            package: package.into(),
            version: version.into(),
            catalog_sha256: digest(catalog),
            executable_sha256: executable_sha256.into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPlan {
    pub schema: String,
    pub operation: String,
    pub binding: Binding,
    pub plan: Value,
}
impl LocalPlan {
    pub fn new(operation: String, binding: Binding, plan: Value) -> Self {
        Self {
            schema: "zix-local-operation/v1".into(),
            operation,
            binding,
            plan,
        }
    }
    pub fn digest(&self) -> String {
        digest(&serde_json::to_value(self).expect("serializable plan"))
    }
    pub fn matches(&self, operation: &str, binding: &Binding, approved: &str) -> bool {
        valid_digest(&self.binding.executable_sha256)
            && self.schema == "zix-local-operation/v1"
            && self.operation == operation
            && &self.binding == binding
            && valid_digest(approved)
            && self.digest() == approved
    }
}
pub fn valid_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn digest(value: &Value) -> String {
    Sha256::digest(serde_json::to_vec(value).expect("serializable JSON"))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plan_is_bound_to_catalog_version_operation_and_payload() {
        let b = Binding::for_catalog(
            "example",
            "1.0.0",
            &serde_json::json!({"commands":[]}),
            &"a".repeat(64),
        );
        let p = LocalPlan::new(
            "example.apply".into(),
            b.clone(),
            serde_json::json!({"value":1}),
        );
        assert!(p.matches("example.apply", &b, &p.digest()));
        let mut changed = b.clone();
        changed.version = "2.0.0".into();
        assert!(!p.matches("example.apply", &changed, &p.digest()));
        assert!(!p.matches("example.delete", &b, &p.digest()));
        let mut changed = p.clone();
        changed.plan["value"] = serde_json::json!(2);
        assert!(!changed.matches("example.apply", &b, &p.digest()));
        let changed = Binding::for_catalog(
            "example",
            "1.0.0",
            &serde_json::json!({"commands":[1]}),
            &"a".repeat(64),
        );
        assert!(!p.matches("example.apply", &changed, &p.digest()));
    }
}
