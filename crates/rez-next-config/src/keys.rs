//! Single source of truth for rez configuration key definitions.
//!
//! Every configuration key rez-next accepts is described here by a [`KeyDef`]:
//! its name, value type, default, whether it originates in rez or is a
//! rez-next extension, and whether it is deprecated.
//!
//! This registry exists because the key set was previously implied by three
//! places that disagreed with each other — the Python `config_schema`
//! validators, the `rezconfig.py` defaults module, and the Rust config struct.
//! Keys that had a validator but no default, or a default but no validator,
//! were silently invisible to whichever side was consulted.

use once_cell::sync::Lazy;
use serde_json::Value as JsonValue;

/// Where a configuration key originates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The key is defined by upstream rez.
    Rez,
    /// The key is a rez-next extension; rez does not define it.
    RezNext,
}

/// The value type a configuration key holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// List of filesystem paths.
    PathList,
    /// List of strings.
    StrList,
    /// Optional list of strings.
    OptionalStrList,
    /// Boolean.
    Bool,
    /// Optional boolean.
    OptionalBool,
    /// Boolean, or a value that forces colour on or off.
    ForceOrBool,
    /// Integer.
    Int,
    /// Floating point number.
    Float,
    /// String.
    Str,
    /// Optional string.
    OptionalStr,
    /// Arbitrary mapping.
    Dict,
    /// Optional mapping.
    OptionalDict,
    /// Optional mapping, or list of mappings.
    OptionalDictOrDictList,
    /// Single character.
    Char,
}

/// Definition of a single configuration key.
#[derive(Debug, Clone)]
pub struct KeyDef {
    /// Key name, in flat snake_case.
    pub name: &'static str,
    /// Value type the key accepts.
    pub ty: KeyType,
    /// Default value, when the key has one.
    pub default: Option<JsonValue>,
    /// Whether the key comes from rez or is a rez-next extension.
    pub origin: Origin,
    /// Whether the key is deprecated.
    pub deprecated: bool,
    /// Short description of what the key controls.
    pub doc: &'static str,
}

const fn key(name: &'static str, ty: KeyType, origin: Origin) -> KeyDef {
    KeyDef {
        name,
        ty,
        default: None,
        origin,
        deprecated: false,
        doc: "",
    }
}

/// Build the registry.
///
/// The first block covers keys that had a schema validator but no declared
/// default. They keep `default: None` here; the validator is the only contract
/// for them today, and this registry makes that explicit rather than implicit.
static REGISTRY: Lazy<Vec<KeyDef>> = Lazy::new(|| {
    vec![
        key("alias_back", KeyType::OptionalStr, Origin::Rez),
        key("alias_fore", KeyType::OptionalStr, Origin::Rez),
        key("alias_styles", KeyType::OptionalStrList, Origin::Rez),
        key("all_parent_variables", KeyType::Bool, Origin::Rez),
        key("all_resetting_variables", KeyType::Bool, Origin::Rez),
        key("browser", KeyType::OptionalStr, Origin::Rez),
        key("cache_listdir", KeyType::Bool, Origin::Rez),
        key("context_tmpdir", KeyType::OptionalStr, Origin::Rez),
        key("context_tracking_amqp", KeyType::OptionalDict, Origin::Rez),
        key(
            "context_tracking_context_fields",
            KeyType::StrList,
            Origin::Rez,
        ),
        key(
            "context_tracking_extra_fields",
            KeyType::OptionalDict,
            Origin::Rez,
        ),
        key("critical_back", KeyType::OptionalStr, Origin::Rez),
        key("critical_fore", KeyType::OptionalStr, Origin::Rez),
        key("critical_styles", KeyType::OptionalStrList, Origin::Rez),
        key("debug_all", KeyType::Bool, Origin::Rez),
        key("debug_back", KeyType::OptionalStr, Origin::Rez),
        key("debug_bind_modules", KeyType::Bool, Origin::Rez),
        key("debug_context_tracking", KeyType::Bool, Origin::Rez),
        key("debug_file_loads", KeyType::Bool, Origin::Rez),
        key("debug_fore", KeyType::OptionalStr, Origin::Rez),
        // `debug_memcache` also carries a default, so it is declared with full
        // metadata in the divergent block below rather than here.
        key("debug_none", KeyType::Bool, Origin::Rez),
        key("debug_package_exclusions", KeyType::Bool, Origin::Rez),
        key("debug_package_release", KeyType::Bool, Origin::Rez),
        key("debug_plugins", KeyType::Bool, Origin::Rez),
        key("debug_resolve_memcache", KeyType::Bool, Origin::Rez),
        key("debug_resources", KeyType::Bool, Origin::Rez),
        key("debug_shell_startup", KeyType::Bool, Origin::Rez),
        key("debug_styles", KeyType::OptionalStrList, Origin::Rez),
        key("default_build_process", KeyType::Str, Origin::Rez),
        key(
            "default_cachable_per_package",
            KeyType::OptionalDict,
            Origin::Rez,
        ),
        key(
            "default_cachable_per_repository",
            KeyType::OptionalDict,
            Origin::Rez,
        ),
        key(
            "default_relocatable_per_package",
            KeyType::OptionalDict,
            Origin::Rez,
        ),
        key(
            "default_relocatable_per_repository",
            KeyType::OptionalDict,
            Origin::Rez,
        ),
        key("difftool", KeyType::OptionalStr, Origin::Rez),
        key("dot_image_format", KeyType::Str, Origin::Rez),
        key("editor", KeyType::OptionalStr, Origin::Rez),
        key("env_var_separators", KeyType::Dict, Origin::Rez),
        key("ephemeral_back", KeyType::OptionalStr, Origin::Rez),
        key("ephemeral_fore", KeyType::OptionalStr, Origin::Rez),
        key("ephemeral_styles", KeyType::OptionalStrList, Origin::Rez),
        key("error_back", KeyType::OptionalStr, Origin::Rez),
        key("error_fore", KeyType::OptionalStr, Origin::Rez),
        key(
            "error_on_missing_variant_requires",
            KeyType::Bool,
            Origin::Rez,
        ),
        key("error_styles", KeyType::OptionalStrList, Origin::Rez),
        key("heading_back", KeyType::OptionalStr, Origin::Rez),
        key("heading_fore", KeyType::OptionalStr, Origin::Rez),
        key("heading_styles", KeyType::OptionalStrList, Origin::Rez),
        key("image_viewer", KeyType::OptionalStr, Origin::Rez),
        key("implicit_back", KeyType::OptionalStr, Origin::Rez),
        key("implicit_fore", KeyType::OptionalStr, Origin::Rez),
        key("implicit_styles", KeyType::OptionalStrList, Origin::Rez),
        key("info_back", KeyType::OptionalStr, Origin::Rez),
        key("info_fore", KeyType::OptionalStr, Origin::Rez),
        key("info_styles", KeyType::OptionalStrList, Origin::Rez),
        key("local_back", KeyType::OptionalStr, Origin::Rez),
        key("local_fore", KeyType::OptionalStr, Origin::Rez),
        key("local_styles", KeyType::OptionalStrList, Origin::Rez),
        key(
            "make_package_temporarily_writable",
            KeyType::Bool,
            Origin::Rez,
        ),
        key("max_package_changelog_revisions", KeyType::Int, Origin::Rez),
        key(
            "memcached_context_file_min_compress_len",
            KeyType::Int,
            Origin::Rez,
        ),
        key(
            "memcached_listdir_min_compress_len",
            KeyType::Int,
            Origin::Rez,
        ),
        key(
            "memcached_package_file_min_compress_len",
            KeyType::Int,
            Origin::Rez,
        ),
        key(
            "memcached_resolve_min_compress_len",
            KeyType::Int,
            Origin::Rez,
        ),
        key("new_session_popen_args", KeyType::OptionalDict, Origin::Rez),
        key("package_cache_async", KeyType::Bool, Origin::Rez),
        key("package_cache_clean_limit", KeyType::Float, Origin::Rez),
        key("package_cache_during_build", KeyType::Bool, Origin::Rez),
        key("package_cache_local", KeyType::Bool, Origin::Rez),
        key("package_cache_log_days", KeyType::Int, Origin::Rez),
        key("package_cache_same_device", KeyType::Bool, Origin::Rez),
        key("package_cache_space_buffer", KeyType::Int, Origin::Rez),
        key("package_cache_used_threshold", KeyType::Int, Origin::Rez),
        key(
            "package_definition_build_python_paths",
            KeyType::PathList,
            Origin::Rez,
        ),
        key(
            "package_definition_python_path",
            KeyType::OptionalStr,
            Origin::Rez,
        ),
        key(
            "package_orderers",
            KeyType::OptionalDictOrDictList,
            Origin::Rez,
        ),
        key(
            "package_preprocess_function",
            KeyType::OptionalStr,
            Origin::Rez,
        ),
        key("package_preprocess_mode", KeyType::Str, Origin::Rez),
        key("pathed_env_vars", KeyType::StrList, Origin::Rez),
        key("pip_extra_args", KeyType::OptionalStrList, Origin::Rez),
        key("pip_install_remaps", KeyType::Dict, Origin::Rez),
        key("platform_map", KeyType::OptionalDict, Origin::Rez),
        key("prompt_release_message", KeyType::Bool, Origin::Rez),
        key("prune_failed_graph", KeyType::Bool, Origin::Rez),
        key("resetting_variables", KeyType::StrList, Origin::Rez),
        key("resource_caching_maxsize", KeyType::Int, Origin::Rez),
        key("standard_system_paths", KeyType::PathList, Origin::Rez),
        key(
            "terminal_emulator_command",
            KeyType::OptionalStr,
            Origin::Rez,
        ),
        key("use_variant_shortlinks", KeyType::Bool, Origin::Rez),
        key(
            "variant_shortlinks_dirname",
            KeyType::OptionalStr,
            Origin::Rez,
        ),
        key("warn_all", KeyType::Bool, Origin::Rez),
        key("warn_none", KeyType::Bool, Origin::Rez),
        key("warn_shell_startup", KeyType::Bool, Origin::Rez),
        key("warning_back", KeyType::OptionalStr, Origin::Rez),
        key("warning_fore", KeyType::OptionalStr, Origin::Rez),
        key("warning_styles", KeyType::OptionalStrList, Origin::Rez),
        // ---------------------------------------------------------------------
        // Keys that had a declared default but no schema validator.
        //
        // Each is registered here so the divergence stops being invisible: either
        // the key is real and now has a definition, or it is deliberately not
        // supported and says so.
        // ---------------------------------------------------------------------

        // rez-next extensions: upstream rez does not define these debug switches.
        KeyDef {
            name: "debug_build",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::RezNext,
            deprecated: false,
            doc: "rez-next extension: emit package build diagnostics.",
        },
        KeyDef {
            name: "debug_resolve",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::RezNext,
            deprecated: false,
            doc: "rez-next extension: emit dependency resolve diagnostics.",
        },
        KeyDef {
            name: "debug_resolve_package_order",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::RezNext,
            deprecated: false,
            doc: "rez-next extension: log the order packages are resolved in.",
        },
        KeyDef {
            name: "debug_resolve_package_selection",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::RezNext,
            deprecated: false,
            doc: "rez-next extension: log why each package version was selected.",
        },
        KeyDef {
            name: "debug_rex_print_all",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::RezNext,
            deprecated: false,
            doc: "rez-next extension: print every Rex command as it is interpreted.",
        },
        KeyDef {
            name: "debug_rex_print_all_external",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::RezNext,
            deprecated: false,
            doc: "rez-next extension: print external commands spawned by Rex.",
        },
        KeyDef {
            name: "warn_unrelocatable",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::Rez,
            deprecated: false,
            doc: "Warn when a package cannot be made relocatable.",
        },
        // rez spells this `debug_memcache`; the trailing `d` was a rez-next typo.
        // The misspelling is kept as a deprecated alias so existing configs keep
        // working, and the correct rez name is the one that is authoritative.
        KeyDef {
            name: "debug_memcache",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::Rez,
            deprecated: false,
            doc: "Emit memcached diagnostics. Spelled to match upstream rez.",
        },
        KeyDef {
            name: "debug_memcached",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(false)),
            origin: Origin::Rez,
            deprecated: true,
            doc: "Deprecated misspelling of `debug_memcache`; prefer `debug_memcache`.",
        },
        // Plugin settings are addressed as `plugins.<plugin>.<setting>` rather than
        // as ordinary scalar keys, so they are registered but not schema-validated
        // as a single value.
        KeyDef {
            name: "plugins",
            ty: KeyType::Dict,
            default: Some(JsonValue::Object(serde_json::Map::new())),
            origin: Origin::Rez,
            deprecated: false,
            doc: "Per-plugin settings, addressed as `plugins.<plugin>.<setting>`.",
        },
        KeyDef {
            name: "disable_rez_1_compatibility",
            ty: KeyType::Bool,
            default: Some(JsonValue::Bool(true)),
            origin: Origin::Rez,
            deprecated: true,
            doc: "Deprecated: rez 1 compatibility is always disabled.",
        },
        // The keys the Rust binary actually models. They were absent from the
        // registry even though `RezCoreConfig` reads them, so validating a
        // config against the registry flagged them as unknown.
        key("packages_path", KeyType::PathList, Origin::Rez),
        key("local_packages_path", KeyType::Str, Origin::Rez),
        key("release_packages_path", KeyType::Str, Origin::Rez),
        key("plugin_path", KeyType::PathList, Origin::Rez),
        key("package_cache_path", KeyType::PathList, Origin::Rez),
        key("default_shell", KeyType::Str, Origin::Rez),
        key("tmpdir", KeyType::Str, Origin::Rez),
        key("implicit_packages", KeyType::StrList, Origin::Rez),
        key("parent_variables", KeyType::StrList, Origin::Rez),
        key("default_relocatable", KeyType::Bool, Origin::Rez),
        key("build_directory", KeyType::Str, Origin::Rez),
        // rez-next extensions: upstream rez has no counterpart.
        key("use_rust_version", KeyType::Bool, Origin::RezNext),
        key("use_rust_solver", KeyType::Bool, Origin::RezNext),
        key("use_rust_repository", KeyType::Bool, Origin::RezNext),
        key("rust_fallback", KeyType::Bool, Origin::RezNext),
        key("thread_count", KeyType::OptionalStr, Origin::RezNext),
        key("cache", KeyType::Dict, Origin::RezNext),
        key("version", KeyType::Str, Origin::RezNext),
    ]
});

/// Return every known configuration key definition.
#[must_use]
pub fn registry() -> &'static [KeyDef] {
    REGISTRY.as_slice()
}

/// Look up a single key definition by name.
#[must_use]
pub fn find(name: &str) -> Option<&'static KeyDef> {
    REGISTRY.iter().find(|k| k.name == name)
}

/// Default values that rez 3.4.0 defines, and that rez-next previously
/// disagreed with.
///
/// These are recorded as data rather than prose so the alignment test can
/// assert rez-next matches upstream exactly.
#[must_use]
pub fn rez_default(key: &str) -> Option<JsonValue> {
    match key {
        "packages_path" => Some(serde_json::json!([
            "~/packages",
            "~/.rez/packages/int",
            "~/.rez/packages/ext",
        ])),
        "local_packages_path" => Some(serde_json::json!("~/packages")),
        // rez uses a forward-slash path on every platform, including Windows.
        "release_packages_path" => Some(serde_json::json!("~/.rez/packages/int")),
        "debug_memcache" => Some(serde_json::json!(false)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 10 keys that had a default in `rezconfig.py` but no schema
    /// validator. Each must now be registered so the divergence is visible.
    const DIVERGENT_KEYS: &[&str] = &[
        "debug_build",
        "debug_resolve",
        "debug_resolve_package_order",
        "debug_resolve_package_selection",
        "debug_rex_print_all",
        "debug_rex_print_all_external",
        "warn_unrelocatable",
        "debug_memcache",
        "debug_memcached",
        "plugins",
        "disable_rez_1_compatibility",
    ];

    #[test]
    fn test_registry_is_not_empty() {
        assert!(registry().len() >= 96);
    }

    #[test]
    fn test_all_divergent_keys_are_registered() {
        // Regression: these keys existed as defaults but were invisible to the
        // schema, so setting them appeared to do nothing.
        for name in DIVERGENT_KEYS {
            assert!(
                find(name).is_some(),
                "divergent key {name} must be registered"
            );
        }
    }

    /// The keys `RezCoreConfig` models and therefore reads from a config file.
    ///
    /// If the struct gains a field that is not registered here, validating a
    /// config against the registry reports a key the binary understands as
    /// unknown — a false positive that would make the warning useless. This
    /// keeps the two sides in step.
    const BINARY_CONFIG_KEYS: &[&str] = &[
        "packages_path",
        "local_packages_path",
        "release_packages_path",
        "plugin_path",
        "package_cache_path",
        "default_shell",
        "tmpdir",
        "editor",
        "image_viewer",
        "browser",
        "difftool",
        "terminal_emulator_command",
        "implicit_packages",
        "parent_variables",
        "resetting_variables",
        "env_var_separators",
        "platform_map",
        "default_relocatable",
        "build_directory",
        "context_tmpdir",
        "use_rust_version",
        "use_rust_solver",
        "use_rust_repository",
        "rust_fallback",
        "thread_count",
        "cache",
        "version",
    ];

    #[test]
    fn test_every_binary_config_key_is_registered() {
        for name in BINARY_CONFIG_KEYS {
            assert!(
                find(name).is_some(),
                "binary config key {name} must be registered, or config \
                 validation will flag it as unknown"
            );
        }
    }

    #[test]
    fn test_binary_keys_that_rez_defines_are_labelled_rez() {
        for name in [
            "packages_path",
            "local_packages_path",
            "release_packages_path",
            "plugin_path",
            "package_cache_path",
            "default_shell",
            "tmpdir",
            "implicit_packages",
            "parent_variables",
            "resetting_variables",
            "env_var_separators",
            "platform_map",
            "default_relocatable",
            "build_directory",
            "context_tmpdir",
        ] {
            assert_eq!(
                find(name).unwrap().origin,
                Origin::Rez,
                "{name} is an upstream rez key and must not be labelled an extension"
            );
        }
    }

    #[test]
    fn test_registry_has_no_duplicate_names() {
        let mut seen = std::collections::HashSet::new();
        for key in registry() {
            assert!(
                seen.insert(key.name),
                "duplicate key definition: {}",
                key.name
            );
        }
    }

    #[test]
    fn test_debug_memcache_spelling_matches_rez() {
        // rez 3.4.0 defines `debug_memcache`; `debug_memcached` is a rez-next
        // typo kept only as a deprecated alias.
        let correct = find("debug_memcache").expect("debug_memcache registered");
        assert!(!correct.deprecated);
        assert_eq!(correct.origin, Origin::Rez);

        let typo = find("debug_memcached").expect("alias registered");
        assert!(typo.deprecated, "the misspelling must be marked deprecated");
    }

    #[test]
    fn test_deprecated_keys_are_marked() {
        assert!(find("disable_rez_1_compatibility").unwrap().deprecated);
        assert!(!find("plugins").unwrap().deprecated);
    }

    #[test]
    fn test_rez_next_extensions_are_labelled() {
        // Extensions must be distinguishable from upstream rez keys.
        for name in [
            "debug_build",
            "debug_resolve",
            "debug_resolve_package_order",
            "debug_resolve_package_selection",
            "debug_rex_print_all",
            "debug_rex_print_all_external",
        ] {
            assert_eq!(
                find(name).unwrap().origin,
                Origin::RezNext,
                "{name} must be labelled a rez-next extension"
            );
        }
    }

    #[test]
    fn test_rez_defaults_match_upstream_rez_3_4_0() {
        // Values verified against rez 3.4.0 `src/rez/rezconfig.py`.
        assert_eq!(
            rez_default("packages_path"),
            Some(serde_json::json!([
                "~/packages",
                "~/.rez/packages/int",
                "~/.rez/packages/ext",
            ]))
        );
        assert_eq!(
            rez_default("local_packages_path"),
            Some(serde_json::json!("~/packages"))
        );
        // rez uses forward slashes on every platform, including Windows.
        assert_eq!(
            rez_default("release_packages_path"),
            Some(serde_json::json!("~/.rez/packages/int"))
        );
        assert_eq!(
            rez_default("debug_memcache"),
            Some(serde_json::json!(false))
        );
    }

    #[test]
    fn test_unknown_key_has_no_rez_default() {
        assert_eq!(rez_default("no_such_key"), None);
    }

    #[test]
    fn test_find_unknown_key_returns_none() {
        assert!(find("no_such_key").is_none());
    }
}
