use anyhow::Result;
use schemars::SchemaGenerator;
use schemars::schema_for;

/// Builds a JSON Schema for the `limits` map whose keys are constrained
/// to the set of language names known to tokei.
pub(crate) fn language_limits_schema(generator: &mut SchemaGenerator) -> schemars::Schema {
    let value_schema = generator.subschema_for::<u64>();

    let names: Vec<serde_json::Value> = tokei::LanguageType::list()
        .iter()
        .map(|(lt, _)| serde_json::Value::String(lt.name().to_owned()))
        .collect();

    let mut schema: schemars::Schema = serde_json::json!({
        "description": "Per-language line limits. Keys are tokei language names (e.g. \"Rust\", \"Python\").",
        "type": "object",
        "propertyNames": { "enum": names },
        "additionalProperties": value_schema.as_value(),
    })
    .try_into()
    .expect("valid schema object");

    // Drop the nested $defs that subschema_for may have produced — the
    // top-level schema already carries definitions.
    schema.remove("$defs");
    schema
}

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
    fn limits_property_names_constrained_to_tokei_languages() {
        let output = generate().expect("generate");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("valid json");
        let limits = parsed
            .pointer("/properties/limits/propertyNames/enum")
            .expect("propertyNames enum");
        let names = limits.as_array().expect("array");
        assert!(names.len() > 100, "should list many languages");
        let has = |name: &str| names.iter().any(|v| v.as_str() == Some(name));
        assert!(has("Rust"), "should contain Rust");
        assert!(has("Python"), "should contain Python");
        assert!(has("JavaScript"), "should contain JavaScript");
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
