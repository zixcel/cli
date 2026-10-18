use crate::{Error, Result, Rule};
use std::collections::BTreeMap;
impl Rule {
    pub(crate) fn validate(&self, value: &str) -> Result<()> {
        let valid = match self {
            Self::Uuid => {
                value.len() == 36
                    && value.bytes().enumerate().all(|(i, b)| {
                        if [8, 13, 18, 23].contains(&i) {
                            b == b'-'
                        } else {
                            b.is_ascii_hexdigit() && !b.is_ascii_uppercase()
                        }
                    })
            }
            Self::Hex32 => {
                value.len() == 32
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            }
            Self::DecimalIdentifier => {
                !value.is_empty() && value.len() <= 30 && value.bytes().all(|b| b.is_ascii_digit())
            }
            Self::Identifier => {
                !value.is_empty()
                    && value.len() <= 128
                    && value != "."
                    && value != ".."
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.@".contains(&b))
            }
        };
        if !valid {
            return Err(Error("invalid_scope_argument"));
        }
        Ok(())
    }
}
pub(crate) fn substitute(template: &str, values: &BTreeMap<String, String>) -> Result<String> {
    let mut output = template.to_owned();
    for (key, value) in values {
        output = output.replace(&format!("{{{key}}}"), value);
    }
    if output.contains(['{', '}']) || output.len() > 2048 {
        return Err(Error("unbound_api_template"));
    }
    Ok(output)
}
pub(crate) fn pointer(value: &str) -> bool {
    value.len() <= 256
        && (value.is_empty() || value.starts_with('/'))
        && !value.chars().any(char::is_control)
}
