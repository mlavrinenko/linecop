use schemars::schema_for;

/// Generates a JSON Schema string for the linecop config format.
///
/// # Panics
///
/// Panics if the schema cannot be serialized to JSON (should never happen).
pub fn generate() -> String {
    let schema = schema_for!(crate::config::Config);
    serde_json::to_string_pretty(&schema).expect("schema serialization cannot fail")
}

#[cfg(test)]
mod tests {
    use super::generate;

    #[test]
    fn generates_valid_json() {
        let output = generate();
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        assert!(parsed.is_object());
    }

    #[test]
    fn schema_has_limits_property() {
        let output = generate();
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        let props = parsed.get("properties").expect("properties");
        assert!(props.get("limits").is_some());
    }

    #[test]
    fn schema_has_count_mode_property() {
        let output = generate();
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        let props = parsed.get("properties").expect("properties");
        assert!(props.get("count_mode").is_some());
    }
}
