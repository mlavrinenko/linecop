use anyhow::Result;
use schemars::schema_for;

/// Generates a JSON Schema string for the linecop config format.
///
/// # Errors
///
/// Returns an error if the schema cannot be serialized to JSON.
pub fn generate() -> Result<String> {
    let schema = schema_for!(crate::config::Config);
    let json = serde_json::to_string_pretty(&schema)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::generate;

    #[test]
    fn generates_valid_json() {
        let output = generate().expect("generate");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        assert!(parsed.is_object());
    }

    #[test]
    fn schema_has_limits_property() {
        let output = generate().expect("generate");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        let props = parsed.get("properties").expect("properties");
        assert!(props.get("limits").is_some());
    }

    #[test]
    fn schema_has_count_mode_property() {
        let output = generate().expect("generate");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        let props = parsed.get("properties").expect("properties");
        assert!(props.get("count_mode").is_some());
    }

    #[test]
    fn committed_schema_is_in_sync() {
        let generated = generate().expect("generate");
        let committed =
            std::fs::read_to_string("linecop-schema.json").expect("read committed schema");
        assert_eq!(
            generated.trim(),
            committed.trim(),
            "linecop-schema.json is out of sync — run `just schema` to regenerate"
        );
    }
}
