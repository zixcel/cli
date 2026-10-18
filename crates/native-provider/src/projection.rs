use crate::{Error, Provider, Request, Result, validation};
use serde_json::{Map, Value, json};
use zix_cli::api_spec::json;
impl Provider {
    pub fn project(&self, request: &Request, bytes: &[u8]) -> Result<Value> {
        let op = self.operation(request)?;
        let raw = json::parse(bytes, 1024 * 1024)?;
        if op
            .projection
            .success_pointer
            .as_ref()
            .is_some_and(|p| raw.pointer(p) != Some(&Value::Bool(true)))
        {
            return Err(Error("provider_response_rejected"));
        }
        for check in &op.identity {
            let expected = validation::substitute(&check.expected, &request.arguments)?;
            if raw.pointer(&check.pointer).and_then(Value::as_str) != Some(expected.as_str()) {
                return Err(Error("provider_identity_mismatch"));
            }
        }
        let selected = raw
            .pointer(&op.projection.pointer)
            .ok_or(Error("invalid_provider_response"))?;
        let row = |item: &Value| -> Result<Value> {
            if !item.is_object() {
                return Err(Error("invalid_provider_response"));
            }
            let mut output = Map::new();
            for (name, pointer) in &op.projection.fields {
                let v = item.pointer(pointer).cloned().unwrap_or(Value::Null);
                if (v.is_null() && op.projection.required.contains(name))
                    || v.is_array()
                    || v.is_object()
                    || v.as_str().is_some_and(|s| s.len() > 4096)
                {
                    return Err(Error("invalid_provider_response"));
                }
                output.insert(name.clone(), v);
            }
            Ok(Value::Object(output))
        };
        let data = if op.projection.collection {
            let rows = selected
                .as_array()
                .filter(|a| a.len() <= 100)
                .ok_or(Error("invalid_provider_response"))?;
            Value::Array(rows.iter().map(row).collect::<Result<Vec<_>>>()?)
        } else {
            row(selected)?
        };
        let value = json!({"schema":"zix-native-observation/v1","provider":self.policy.namespace,
            "operation":request.operation,"data":data,"completeness":"single_response",
            "permission_state":"not_inferred","request_cost":"not_assessed"});
        if serde_json::to_vec(&value)
            .map_err(|_| Error("invalid_provider_response"))?
            .len()
            + 512
            > request.limits.maximum_response_bytes
        {
            return Err(Error("provider_output_limit"));
        }
        Ok(value)
    }
}
