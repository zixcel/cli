//! Select one cache string without allocating the unselected refresh tokens.
use crate::{Error, Result};
use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, Visitor};
use zeroize::Zeroizing;
struct Field<'a>(&'a str);
impl<'de> DeserializeSeed<'de> for Field<'_> {
    type Value = Zeroizing<String>;
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        d: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for Field<'_> {
    type Value = Zeroizing<String>;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("credential cache object")
    }
    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut value = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == self.0 {
                if value.is_some() {
                    return Err(serde::de::Error::custom("duplicate field"));
                }
                value = Some(Zeroizing::new(map.next_value::<String>()?));
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        value.ok_or_else(|| serde::de::Error::custom("missing field"))
    }
}
pub(super) fn select(bytes: &[u8], field: Option<&str>) -> Result<Zeroizing<String>> {
    let value = if let Some(field) = field {
        let mut d = serde_json::Deserializer::from_slice(bytes);
        let value = Field(field)
            .deserialize(&mut d)
            .map_err(|_| Error("invalid_credential_cache"))?;
        d.end().map_err(|_| Error("invalid_credential_cache"))?;
        value
    } else {
        Zeroizing::new(
            std::str::from_utf8(bytes)
                .map_err(|_| Error("invalid_credential_environment"))?
                .to_owned(),
        )
    };
    if value.is_empty()
        || value.len() > 16384
        || !value.is_ascii()
        || value.bytes().any(|b| b.is_ascii_control())
    {
        return Err(Error("invalid_credential_environment"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_only_access_token_and_rejects_ambiguous_or_unsafe_values() {
        let cache = br#"{"refresh_token":"never-forward","access_token":"access-only","nested":{"token":1}}"#;
        assert_eq!(
            &**select(cache, Some("access_token")).unwrap(),
            "access-only"
        );
        for invalid in [
            br#"{"access_token":"a","access_token":"b"}"#.as_slice(),
            br#"{"access_token":4}"#,
            br#"{}"#,
            br#"{"access_token":"a\nb"}"#,
            br#"{"access_token":"a"} {}"#,
        ] {
            assert!(select(invalid, Some("access_token")).is_err());
        }
        assert_eq!(&**select(b"raw-token", None).unwrap(), "raw-token");
    }
}
