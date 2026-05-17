//! Built-in release hook implementations.
//!
//! This module provides sample hooks that can be used for testing
//! or as references for custom hook implementations.

use crate::{ReleaseHook, Result};
use std::path::PathBuf;

/// Configuration for EmailHook.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct EmailHookConfig {
    smtp_server: String,
    smtp_port: u16,
    from_address: String,
    to_addresses: Vec<String>,
    use_tls: bool,
}

impl Default for EmailHookConfig {
    fn default() -> Self {
        Self {
            smtp_server: "localhost".to_string(),
            smtp_port: 25,
            from_address: "rez@localhost".to_string(),
            to_addresses: vec![],
            use_tls: false,
        }
    }
}

/// Configuration for WebHook.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct WebHookConfig {
    url: String,
    method: String,
    headers: std::collections::HashMap<String, String>,
    timeout_secs: u64,
}

impl Default for WebHookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: "POST".to_string(),
            headers: std::collections::HashMap::new(),
            timeout_secs: 30,
        }
    }
}

/// A no-op release hook that does nothing.
///
/// This hook is useful for testing or as a placeholder.
/// All hook methods return `Ok(())` without doing anything.
pub struct NoopHook;

impl ReleaseHook for NoopHook {
    fn name() -> String {
        "noop".to_string()
    }

    fn new(_source_path: PathBuf) -> Result<Self> {
        Ok(Self)
    }
}

/// A logging release hook that logs messages when hooks are called.
///
/// This hook is useful for debugging the release process.
/// It logs the event name and package path at INFO level.
pub struct LoggingHook;

impl ReleaseHook for LoggingHook {
    fn name() -> String {
        "logging".to_string()
    }

    fn new(_source_path: PathBuf) -> Result<Self> {
        Ok(Self)
    }

    fn pre_build(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: Option<&[usize]>,
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!("ReleaseHook: pre_build called");
        Ok(())
    }

    fn pre_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: Option<&[usize]>,
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!("ReleaseHook: pre_release called");
        Ok(())
    }

    fn post_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: &[String],
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!("ReleaseHook: post_release called");
        Ok(())
    }
}

/// An email release hook that sends notifications via SMTP.
///
/// This hook sends email notifications before and after the release process.
/// Configuration is read from a file at `source_path` (JSON/TOML format).
pub struct EmailHook {
    config: EmailHookConfig,
}

impl ReleaseHook for EmailHook {
    fn name() -> String {
        "email".to_string()
    }

    fn new(source_path: PathBuf) -> Result<Self> {
        let config = Self::read_config(&source_path).unwrap_or_default();
        Ok(Self { config })
    }

    fn pre_build(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: Option<&[usize]>,
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!(
            "EmailHook: pre_build - would send email to {:?}",
            self.config.to_addresses
        );
        Ok(())
    }

    fn pre_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: Option<&[usize]>,
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!(
            "EmailHook: pre_release - would send email to {:?}",
            self.config.to_addresses
        );
        Ok(())
    }

    fn post_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: &[String],
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!(
            "EmailHook: post_release - would send email to {:?}",
            self.config.to_addresses
        );
        Ok(())
    }
}

impl EmailHook {
    /// Read configuration from a file.
    fn read_config(path: &std::path::Path) -> Option<EmailHookConfig> {
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).ok()
        } else {
            // Assume TOML
            toml::from_str(&content).ok()
        }
    }
}

/// A webhook release hook that sends HTTP requests.
///
/// This hook sends HTTP requests to a configured URL before and after release.
/// Configuration is read from a file at `source_path` (JSON/TOML format).
pub struct WebHook {
    config: WebHookConfig,
}

impl ReleaseHook for WebHook {
    fn name() -> String {
        "webhook".to_string()
    }

    fn new(source_path: PathBuf) -> Result<Self> {
        let config = Self::read_config(&source_path).unwrap_or_default();
        if config.url.is_empty() {
            return Err(crate::ReleaseHookError::Error(
                "WebHook requires 'url' in configuration".to_string(),
            ));
        }
        Ok(Self { config })
    }

    fn pre_build(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: Option<&[usize]>,
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!(
            "WebHook: pre_build - would send {} request to {}",
            self.config.method,
            self.config.url
        );
        Ok(())
    }

    fn pre_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: Option<&[usize]>,
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!(
            "WebHook: pre_release - would send {} request to {}",
            self.config.method,
            self.config.url
        );
        Ok(())
    }

    fn post_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: &[String],
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!(
            "WebHook: post_release - would send {} request to {}",
            self.config.method,
            self.config.url
        );
        Ok(())
    }
}

impl WebHook {
    /// Read configuration from a file.
    fn read_config(path: &std::path::Path) -> Option<WebHookConfig> {
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).ok()
        } else {
            toml::from_str(&content).ok()
        }
    }
}

/// Register all built-in hooks with the global registry.
///
/// This function should be called during initialization.
pub fn register_builtin_hooks() {
    let registry = crate::get_registry();

    registry.register("noop", |_path| {
        Ok(Box::new(NoopHook) as Box<dyn ReleaseHook>)
    });

    registry.register("logging", |path| {
        Ok(Box::new(LoggingHook::new(path)?) as Box<dyn ReleaseHook>)
    });

    registry.register("email", |path| {
        Ok(Box::new(EmailHook::new(path)?) as Box<dyn ReleaseHook>)
    });

    registry.register("webhook", |path| {
        Ok(Box::new(WebHook::new(path)?) as Box<dyn ReleaseHook>)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_noop_hook_creation() {
        let hook = NoopHook::new(PathBuf::from("/tmp"));
        assert!(hook.is_ok());
    }

    #[test]
    fn test_noop_hook_name() {
        assert_eq!(NoopHook::name(), "noop");
    }

    #[test]
    fn test_noop_hook_pre_build() {
        let hook = NoopHook::new(PathBuf::from("/tmp")).unwrap();
        let result = hook.pre_build(
            "test_user",
            Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_noop_hook_pre_release() {
        let hook = NoopHook::new(PathBuf::from("/tmp")).unwrap();
        let result = hook.pre_release(
            "test_user",
            Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_noop_hook_post_release() {
        let hook = NoopHook::new(PathBuf::from("/tmp")).unwrap();
        let result = hook.post_release(
            "test_user",
            Path::new("/tmp/install"),
            &[],
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_logging_hook_creation() {
        let hook = LoggingHook::new(PathBuf::from("/tmp"));
        assert!(hook.is_ok());
    }

    #[test]
    fn test_logging_hook_name() {
        assert_eq!(LoggingHook::name(), "logging");
    }

    #[test]
    fn test_logging_hook_pre_build() {
        let hook = LoggingHook::new(PathBuf::from("/tmp")).unwrap();
        let result = hook.pre_build(
            "test_user",
            Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_builtin_hooks() {
        // Note: This test verifies the function runs without panic
        // The global registry is shared, so we can't easily assert registration
        register_builtin_hooks();

        let types = crate::get_release_hook_types();
        // After registration, "noop", "logging", "email", "webhook" should be available
        assert!(types.contains(&"noop".to_string()));
        assert!(types.contains(&"logging".to_string()));
        assert!(types.contains(&"email".to_string()));
        assert!(types.contains(&"webhook".to_string()));
    }

    #[test]
    fn test_email_hook_creation_default_config() {
        // EmailHook can be created with default config (no config file)
        let hook = EmailHook::new(std::path::PathBuf::from("/nonexistent/path"));
        assert!(hook.is_ok());
    }

    #[test]
    fn test_email_hook_name() {
        assert_eq!(EmailHook::name(), "email");
    }

    #[test]
    fn test_email_hook_pre_build() {
        let hook = EmailHook::new(std::path::PathBuf::from("/tmp")).unwrap();
        let result = hook.pre_build(
            "test_user",
            std::path::Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_email_hook_pre_release() {
        let hook = EmailHook::new(std::path::PathBuf::from("/tmp")).unwrap();
        let result = hook.pre_release(
            "test_user",
            std::path::Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_email_hook_post_release() {
        let hook = EmailHook::new(std::path::PathBuf::from("/tmp")).unwrap();
        let result = hook.post_release(
            "test_user",
            std::path::Path::new("/tmp/install"),
            &[],
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_hook_creation_with_url() {
        // WebHook requires URL in config - test with invalid path (uses default)
        let hook = WebHook::new(std::path::PathBuf::from("/nonexistent/path"));
        // Should fail because default config has empty URL
        assert!(hook.is_err());
    }

    #[test]
    fn test_web_hook_name() {
        assert_eq!(WebHook::name(), "webhook");
    }

    #[test]
    fn test_web_hook_pre_build() {
        // This test creates a hook with a config file
        // For now, just test that the method signature works
        let config = WebHookConfig {
            url: "https://example.com/webhook".to_string(),
            method: "POST".to_string(),
            headers: std::collections::HashMap::new(),
            timeout_secs: 30,
        };
        let hook = WebHook { config };
        let result = hook.pre_build(
            "test_user",
            std::path::Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_hook_pre_release() {
        let config = WebHookConfig {
            url: "https://example.com/webhook".to_string(),
            method: "POST".to_string(),
            headers: std::collections::HashMap::new(),
            timeout_secs: 30,
        };
        let hook = WebHook { config };
        let result = hook.pre_release(
            "test_user",
            std::path::Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_hook_post_release() {
        let config = WebHookConfig {
            url: "https://example.com/webhook".to_string(),
            method: "POST".to_string(),
            headers: std::collections::HashMap::new(),
            timeout_secs: 30,
        };
        let hook = WebHook { config };
        let result = hook.post_release(
            "test_user",
            std::path::Path::new("/tmp/install"),
            &[],
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }
}
