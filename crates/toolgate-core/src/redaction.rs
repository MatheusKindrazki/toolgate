use serde_json::Value;
const REDACTED: &str = "[REDACTED]";
pub fn redact(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        if sensitive_key(k) {
                            Value::String(REDACTED.into())
                        } else {
                            redact(v)
                        },
                    )
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        other => other.clone(),
    }
}
fn sensitive_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "authorization",
        "api_key",
        "apikey",
        "private_key",
        "content",
    ]
    .iter()
    .any(|x| k.contains(x))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn redacts_before_persistence() {
        let result =
            redact(&json!({"token":"abcd", "nested":{"api_key":"x"}, "file_path":"/tmp/a"}));
        assert_eq!(result["token"], REDACTED);
        assert_eq!(result["nested"]["api_key"], REDACTED);
        assert_eq!(result["file_path"], "/tmp/a");
    }
}
