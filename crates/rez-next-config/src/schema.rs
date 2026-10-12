//! JSON Schema export for the rez-next configuration key registry.
//!
//! **rez-next extension**: upstream rez validates its config with a Python
//! schema module and exposes no machine-readable schema. Generating JSON Schema
//! from the [`KeyDef`](crate::keys::KeyDef) registry lets editors and agents
//! validate a `rezconfig` before rez-next ever reads it.

use crate::keys::{KeyDef, KeyType, registry};
use serde_json::{Map, Value as JsonValue, json};

/// The JSON Schema dialect this module emits.
///
/// The draft is pinned so a consumer can select a validator without sniffing
/// the document.
pub const SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Build the full configuration JSON Schema (draft 2020-12).
///
/// Every key in the registry becomes a property. Unknown keys are *not*
/// rejected: rez tolerates plugin settings such as `plugins.<name>.<setting>`,
/// and a schema that refused them would be wrong for real configs.
#[must_use]
pub fn json_schema() -> JsonValue {
    let mut properties = Map::new();

    for key in registry() {
        properties.insert(key.name.to_string(), key_schema(key));
    }

    let mut schema = Map::new();
    schema.insert("$schema".into(), json!(SCHEMA_DIALECT));
    schema.insert(
        "title".into(),
        json!("rez-next configuration (rez-next extension)"),
    );
    schema.insert("type".into(), json!("object"));
    schema.insert("properties".into(), JsonValue::Object(properties));
    // Plugin settings live under `plugins.*` and cannot be enumerated here.
    schema.insert("additionalProperties".into(), json!(true));

    JsonValue::Object(schema)
}

/// Build the JSON Schema fragment for a single key.
fn key_schema(key: &KeyDef) -> JsonValue {
    let mut fragment = type_schema(key.ty);

    let Some(object) = fragment.as_object_mut() else {
        return fragment;
    };

    if let Some(default) = &key.default {
        object.insert("default".into(), default.clone());
    }

    if key.deprecated {
        // Not part of the JSON Schema vocabulary, but a conventional marker
        // that editors surface as strikethrough.
        object.insert("deprecated".into(), json!(true));
    }

    // `x-origin` is always present so a consumer can tell which settings
    // upstream rez would ignore, even for keys that carry no prose.
    object.insert(
        "x-origin".into(),
        json!(match key.origin {
            crate::keys::Origin::Rez => "rez",
            crate::keys::Origin::RezNext => "rez-next",
        }),
    );

    if key.doc.is_empty() {
        object.insert("description".into(), json!(key.name));
    } else {
        object.insert("description".into(), json!(key.doc));
    }

    fragment
}

/// Translate a [`KeyType`] into its JSON Schema type expression.
///
/// Optional variants accept `null` in addition to their base type, because a
/// config that sets `editor: null` is meaningful in rez.
fn type_schema(ty: KeyType) -> JsonValue {
    match ty {
        KeyType::PathList | KeyType::StrList => list_of("string"),
        KeyType::OptionalStrList => nullable(list_of("string")),
        KeyType::Bool => json!({ "type": "boolean" }),
        KeyType::OptionalBool => nullable(json!({ "type": "boolean" })),
        KeyType::ForceOrBool => json!({
            "anyOf": [
                { "type": "boolean" },
                { "type": "string", "enum": ["force", "always", "never"] },
            ]
        }),
        KeyType::Int => json!({ "type": "integer" }),
        KeyType::Float => json!({ "type": "number" }),
        KeyType::Str => json!({ "type": "string" }),
        KeyType::OptionalStr => nullable(json!({ "type": "string" })),
        KeyType::Dict => json!({ "type": "object" }),
        KeyType::OptionalDict => nullable(json!({ "type": "object" })),
        KeyType::OptionalDictOrDictList => nullable(json!({
            "anyOf": [
                { "type": "object" },
                { "type": "array", "items": { "type": "object" } },
            ]
        })),
        KeyType::Char => json!({ "type": "string", "minLength": 1, "maxLength": 1 }),
    }
}

/// Wrap a schema so `null` is also accepted.
fn nullable(schema: JsonValue) -> JsonValue {
    json!({ "anyOf": [schema, { "type": "null" }] })
}

/// A homogeneous array schema.
fn list_of(item_type: &str) -> JsonValue {
    json!({ "type": "array", "items": { "type": item_type } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{Origin, find};

    /// A minimal draft 2020-12 validator: enough that a schema which is
    /// structurally unsound (a property with no type information, or a
    /// `properties` map that disagrees with the registry) fails here.
    fn assert_valid_schema(schema: &JsonValue) {
        assert_eq!(
            schema.get("$schema").and_then(|v| v.as_str()),
            Some(SCHEMA_DIALECT),
            "schema must declare the 2020-12 dialect"
        );
        assert_eq!(
            schema.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "root schema must be an object"
        );

        let properties = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("schema must have a properties object");

        assert_eq!(
            properties.len(),
            registry().len(),
            "every registry key must appear exactly once in properties"
        );

        for (name, fragment) in properties {
            let key = find(name).unwrap_or_else(|| panic!("{name} must be a registry key"));
            assert!(
                fragment.is_object(),
                "{name}: schema fragment must be an object"
            );
            // A fragment is only meaningful if it constrains something.
            let constrains = ["type", "anyOf", "enum"].iter().any(|k| {
                fragment.get(*k).is_some()
                    || fragment
                        .get("anyOf")
                        .and_then(|v| v.as_array())
                        .is_some_and(|a| a.iter().any(|s| s.get(*k).is_some()))
            });
            assert!(constrains, "{name}: fragment must constrain a type");

            if !key.doc.is_empty() {
                assert!(
                    fragment.get("description").is_some(),
                    "{name}: must carry its description"
                );
            }
            if key.deprecated {
                assert_eq!(
                    fragment.get("deprecated"),
                    Some(&json!(true)),
                    "{name}: must be marked deprecated"
                );
            }
            if let Some(default) = &key.default {
                assert_eq!(
                    fragment.get("default"),
                    Some(default),
                    "{name}: must carry its default"
                );
            }
        }
    }

    #[test]
    fn test_schema_is_loadable() {
        assert_valid_schema(&json_schema());
    }

    #[test]
    fn test_schema_covers_every_registry_key() {
        let schema = json_schema();
        let properties = schema["properties"].as_object().expect("properties");
        for key in registry() {
            assert!(
                properties.contains_key(key.name),
                "registry key {} missing from schema",
                key.name
            );
        }
    }

    #[test]
    fn test_schema_marks_rez_next_extensions() {
        // Extensions must be distinguishable from upstream rez keys so a
        // consumer can tell which settings upstream rez would ignore.
        let schema = json_schema();
        let debug_build = &schema["properties"]["debug_build"];
        assert_eq!(debug_build["x-origin"], json!("rez-next"));

        let editor = &schema["properties"]["editor"];
        assert_eq!(editor["x-origin"], json!("rez"));
        assert_eq!(find("editor").unwrap().origin, Origin::Rez);
    }

    #[test]
    fn test_optional_types_accept_null() {
        let schema = json_schema();
        let properties = schema["properties"].as_object().expect("properties");

        // `editor` is OptionalStr: `editor: null` is a meaningful rez setting.
        let editor = &properties["editor"];
        assert!(
            editor["anyOf"]
                .as_array()
                .is_some_and(|a| a.iter().any(|s| s["type"] == "null")),
            "OptionalStr must accept null: {editor}"
        );

        // `packages_path` is a PathList, so null is not a valid value.
        let packages_path = &properties["standard_system_paths"];
        assert_eq!(packages_path["type"], json!("array"));
        assert_eq!(packages_path["items"]["type"], json!("string"));
    }

    #[test]
    fn test_char_type_is_single_character() {
        // Char keys are rare; assert the constraint explicitly so the mapping
        // is covered even though no registry key currently uses it.
        let schema = type_schema(KeyType::Char);
        assert_eq!(schema["minLength"], json!(1));
        assert_eq!(schema["maxLength"], json!(1));
    }

    #[test]
    fn test_every_key_type_maps_to_a_schema() {
        // Guards the mapping table: adding a KeyType variant without a schema
        // branch would otherwise produce an empty, silently permissive fragment.
        let variants = [
            KeyType::PathList,
            KeyType::StrList,
            KeyType::OptionalStrList,
            KeyType::Bool,
            KeyType::OptionalBool,
            KeyType::ForceOrBool,
            KeyType::Int,
            KeyType::Float,
            KeyType::Str,
            KeyType::OptionalStr,
            KeyType::Dict,
            KeyType::OptionalDict,
            KeyType::OptionalDictOrDictList,
            KeyType::Char,
        ];
        for ty in variants {
            let schema = type_schema(ty);
            assert!(
                schema.get("type").is_some() || schema.get("anyOf").is_some(),
                "{ty:?} must map to a schema with type information"
            );
        }
    }

    #[test]
    fn test_schema_allows_additional_properties() {
        // Plugin settings are addressed as `plugins.<plugin>.<setting>` and
        // cannot be enumerated, so unknown keys must not be rejected.
        assert_eq!(json_schema()["additionalProperties"], json!(true));
    }
}
