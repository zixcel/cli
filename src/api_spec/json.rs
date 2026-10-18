use crate::{Error, Result};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Json;
        impl<'de> Visitor<'de> for Json {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("JSON with unique keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::String(v.into())))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(v)) = a.next_element()? {
                    values.push(v);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = Map::new();
                while let Some((k, Unique(v))) = a.next_entry::<String, Unique>()? {
                    if values.insert(k, v).is_some() {
                        return Err(de::Error::custom("duplicate key"));
                    }
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        d.deserialize_any(Json)
    }
}
pub fn parse(bytes: &[u8], limit: usize) -> Result<Value> {
    if bytes.len() > limit {
        return Err(Error("api_input_too_large"));
    }
    // First reject duplicate keys; then preserve exact decimal lexemes in Value.
    let Unique(_) = serde_json::from_slice(bytes).map_err(|_| Error("invalid_api_json"))?;
    let value = serde_json::from_slice(bytes).map_err(|_| Error("invalid_api_json"))?;
    let mut nodes = 0;
    inspect(&value, 0, &mut nodes)?;
    Ok(value)
}
fn inspect(value: &Value, depth: usize, nodes: &mut usize) -> Result<()> {
    *nodes += 1;
    if depth > 64 || *nodes > 200_000 {
        return Err(Error("api_structure_limit"));
    }
    match value {
        Value::Array(a) => {
            for v in a {
                inspect(v, depth + 1, nodes)?;
            }
        }
        Value::Object(o) => {
            for v in o.values() {
                inspect(v, depth + 1, nodes)?;
            }
        }
        _ => {}
    }
    Ok(())
}
