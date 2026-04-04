// ── Phase 92: Binary format (rxtb) tests ─────────────────────────────────────
#[cfg(test)]
mod rxtb_tests {
    use crate::{
        serialization::{ContextFormat, ContextSerializer},
        ContextStatus, ResolvedContext,
    };
    use rez_next_package::Package;
    use rez_next_version::Version;
    use tempfile::TempDir;

    fn make_ctx(packages: &[(&str, &str)]) -> ResolvedContext {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        for (name, ver) in packages {
            let mut pkg = Package::new(name.to_string());
            pkg.version = Some(Version::parse(ver).unwrap());
            ctx.resolved_packages.push(pkg);
        }
        ctx
    }

    /// rxtb file format: serialize → write file → load → same packages
    #[tokio::test]
    async fn test_rxtb_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ctx.rxtb");

        let ctx = make_ctx(&[("maya", "2024.0"), ("python", "3.10.0")]);
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Binary)
            .await
            .unwrap();

        assert!(path.exists(), "rxtb file should be created");
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(
            loaded.resolved_packages.len(),
            2,
            "Should reload 2 packages from rxtb"
        );
        let names: Vec<_> = loaded
            .resolved_packages
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(names.contains(&"maya"), "maya should be in loaded packages");
        assert!(
            names.contains(&"python"),
            "python should be in loaded packages"
        );
    }

    /// rxtb roundtrip: packages and env_vars survive serialize → deserialize
    #[test]
    fn test_rxtb_serialize_deserialize_roundtrip() {
        let mut ctx = make_ctx(&[("nuke", "14.0"), ("ocio", "2.2")]);
        ctx.environment_vars
            .insert("OCIO".to_string(), "/opt/ocio/config.ocio".to_string());
        ctx.environment_vars
            .insert("NUKE_PATH".to_string(), "/opt/nuke/14.0".to_string());

        let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Binary).unwrap();
        assert!(!bytes.is_empty(), "Serialized bytes should not be empty");

        let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Binary).unwrap();
        assert_eq!(restored.resolved_packages.len(), 2);
        assert_eq!(
            restored.environment_vars.get("OCIO"),
            ctx.environment_vars.get("OCIO")
        );
        assert_eq!(
            restored.environment_vars.get("NUKE_PATH"),
            ctx.environment_vars.get("NUKE_PATH")
        );
    }

    /// from_string / to_string with Binary format uses base64 encoding
    #[test]
    fn test_rxtb_to_string_is_base64() {
        let ctx = make_ctx(&[("houdini", "20.5")]);
        let b64_str = ContextSerializer::to_string(&ctx, ContextFormat::Binary).unwrap();

        // base64 strings only contain A-Z, a-z, 0-9, +, /, =
        let is_base64 = b64_str
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '\n');
        assert!(
            is_base64,
            "Binary format to_string should return base64: {}",
            &b64_str[..b64_str.len().min(50)]
        );
    }

    /// from_string roundtrip with Binary format
    #[test]
    fn test_rxtb_from_string_roundtrip() {
        let ctx = make_ctx(&[("renderman", "25.0"), ("katana", "6.0")]);
        let b64 = ContextSerializer::to_string(&ctx, ContextFormat::Binary).unwrap();
        let restored = ContextSerializer::from_string(&b64, ContextFormat::Binary).unwrap();

        let names: Vec<_> = restored
            .resolved_packages
            .iter()
            .map(|p| p.name.clone())
            .collect();
        assert!(
            names.contains(&"renderman".to_string()),
            "renderman should survive binary roundtrip"
        );
        assert!(
            names.contains(&"katana".to_string()),
            "katana should survive binary roundtrip"
        );
    }

    /// ContextFormat extension detection for rxtb
    #[test]
    fn test_rxtb_format_detection() {
        let path = std::path::Path::new("mycontext.rxtb");
        let fmt = ContextFormat::from_extension(path);
        assert_eq!(
            fmt,
            Some(ContextFormat::Binary),
            "rxtb should be detected as Binary"
        );

        let path2 = std::path::Path::new("mycontext.rxt");
        let fmt2 = ContextFormat::from_extension(path2);
        assert_eq!(
            fmt2,
            Some(ContextFormat::Json),
            "rxt should be detected as Json"
        );
    }

    /// JSON format extension is still "rxt", binary is "rxtb"
    #[test]
    fn test_format_extension_names() {
        assert_eq!(ContextFormat::Json.extension(), "rxt");
        assert_eq!(ContextFormat::Binary.extension(), "rxtb");
    }

    /// Empty context serializes to binary and back
    #[test]
    fn test_rxtb_empty_context() {
        let ctx = ResolvedContext::from_requirements(vec![]);
        let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Binary).unwrap();
        let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Binary).unwrap();
        assert_eq!(restored.resolved_packages.len(), 0);
        // Empty context may or may not have environment vars depending on implementation
        let _ = restored.environment_vars;
    }

    /// Binary format produces smaller or equal bytes vs JSON pretty (no forced assertion, just no panic)
    #[test]
    fn test_binary_vs_json_both_valid() {
        let ctx = make_ctx(&[("pkg_a", "1.0"), ("pkg_b", "2.0"), ("pkg_c", "3.0")]);
        let json_bytes = ContextSerializer::serialize(&ctx, ContextFormat::Json).unwrap();
        let bin_bytes = ContextSerializer::serialize(&ctx, ContextFormat::Binary).unwrap();
        assert!(!json_bytes.is_empty(), "JSON bytes non-empty");
        assert!(!bin_bytes.is_empty(), "Binary bytes non-empty");
    }
}
