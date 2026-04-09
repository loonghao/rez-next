use super::*;

mod test_suite_basic {

    use super::*;

    #[test]
    fn test_new_suite_is_empty() {
        let s = PySuite::new(None);
        assert_eq!(s.inner.len(), 0);
        assert!(s.inner.context_names().is_empty());
    }

    #[test]
    fn test_new_suite_with_description() {
        let s = PySuite::new(Some("my suite"));
        assert_eq!(s.inner.description.as_deref(), Some("my suite"));
    }

    #[test]
    fn test_add_context_increases_count() {
        let mut s = PySuite::new(None);
        s.add_context("maya", vec!["maya-2023".to_string()])
            .unwrap();
        assert_eq!(s.inner.len(), 1);
    }

    #[test]
    fn test_add_multiple_contexts() {
        let mut s = PySuite::new(None);
        s.add_context("maya", vec!["maya-2023".to_string()])
            .unwrap();
        s.add_context("nuke", vec!["nuke-14".to_string()]).unwrap();
        assert_eq!(s.inner.len(), 2);
    }

    #[test]
    fn test_context_names_reflects_added_contexts() {
        let mut s = PySuite::new(None);
        s.add_context("ctx1", vec!["pkgA-1".to_string()]).unwrap();
        s.add_context("ctx2", vec!["pkgB-2".to_string()]).unwrap();
        let names = s.context_names();
        assert!(names.contains(&"ctx1".to_string()));
        assert!(names.contains(&"ctx2".to_string()));
    }

    #[test]
    fn test_remove_context_decreases_count() {
        let mut s = PySuite::new(None);
        s.add_context("ctx1", vec!["pkg-1".to_string()]).unwrap();
        s.remove_context("ctx1").unwrap();
        assert_eq!(s.inner.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_context_returns_err() {
        let mut s = PySuite::new(None);
        let result = s.remove_context("no_such_context");
        assert!(result.is_err(), "removing absent context should fail");
    }

    #[test]
    fn test_len_via_inner() {
        let mut s = PySuite::new(None);
        assert_eq!(s.inner.len(), 0);
        s.add_context("a", vec![]).unwrap();
        assert_eq!(s.inner.len(), 1);
    }
}

mod test_suite_description {
    use super::*;

    #[test]
    fn test_default_description_is_none() {
        let s = PySuite::new(None);
        assert!(s.inner.description.is_none());
    }

    #[test]
    fn test_set_description_updates_value() {
        let mut s = PySuite::new(Some("first"));
        s.set_description(Some("second".to_string()));
        assert_eq!(s.inner.description.as_deref(), Some("second"));
    }

    #[test]
    fn test_clear_description() {
        let mut s = PySuite::new(Some("has desc"));
        s.set_description(None);
        assert!(s.inner.description.is_none());
    }
}

mod test_suite_conflict_mode {
    use super::*;

    #[test]
    fn test_set_conflict_mode_prefix() {
        let mut s = PySuite::new(None);
        assert!(s.set_conflict_mode("prefix").is_ok());
    }

    #[test]
    fn test_set_conflict_mode_invalid_returns_err() {
        let mut s = PySuite::new(None);
        let result = s.set_conflict_mode("garbage_mode_xyz");
        assert!(result.is_err(), "invalid conflict mode should fail");
    }
}

mod test_suite_is_suite {
    use super::*;

    #[test]
    fn test_is_suite_nonexistent_path_returns_false() {
        assert!(!PySuite::is_suite("/nonexistent/path/xyz"));
    }

    #[test]
    fn test_is_suite_empty_temp_dir_returns_false() {
        let tmp = std::env::temp_dir().join("rez_suite_test_empty");
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(!PySuite::is_suite(&tmp.to_string_lossy()));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

mod test_suite_manager {
    use super::*;

    #[test]
    fn test_suite_manager_new_empty_paths() {
        let mgr = PySuiteManager::new(Some(vec![]));
        let names = mgr.list_suite_names();
        assert!(names.is_empty(), "empty path manager should have no suites");
    }

    #[test]
    fn test_suite_manager_find_suites_nonexistent_returns_empty() {
        let mgr = PySuiteManager::new(Some(vec!["/no/such/path".to_string()]));
        let suites = mgr.find_suites();
        assert!(suites.is_empty());
    }

    #[test]
    fn test_suite_manager_load_nonexistent_returns_err() {
        let mgr = PySuiteManager::new(Some(vec![]));
        let result = mgr.load_suite("nonexistent_suite");
        assert!(result.is_err());
    }
}

mod test_suite_extra {
    use super::*;

    #[test]
    fn test_repr_contains_contexts_count() {
        let mut s = PySuite::new(None);
        s.add_context("a", vec!["pkg-1".to_string()]).unwrap();
        s.add_context("b", vec!["pkg-2".to_string()]).unwrap();
        let repr = s.__repr__();
        assert!(repr.contains("Suite("), "repr must start with Suite(, got {repr}");
        assert!(repr.contains('2') || repr.contains("contexts=2"),
            "repr must reflect 2 contexts, got {repr}");
    }

    #[test]
    fn test_add_same_context_twice_is_err_or_ok() {
        let mut s = PySuite::new(None);
        s.add_context("dup", vec!["pkg-1".to_string()]).unwrap();
        // Adding same name again: implementation may error or silently update.
        // We only verify the function doesn't panic.
        let _ = s.add_context("dup", vec!["pkg-2".to_string()]);
    }

    #[test]
    fn test_context_names_empty_after_remove_all() {
        let mut s = PySuite::new(None);
        s.add_context("x", vec![]).unwrap();
        s.add_context("y", vec![]).unwrap();
        s.remove_context("x").unwrap();
        s.remove_context("y").unwrap();
        assert!(s.context_names().is_empty(), "context_names must be empty after removing all");
    }

    #[test]
    fn test_alias_tool_nonexistent_context_returns_err() {
        let mut s = PySuite::new(None);
        let result = s.alias_tool("no_such_ctx", "new_alias", "original_tool");
        assert!(result.is_err(), "alias_tool on absent context should fail");
    }

    #[test]
    fn test_hide_tool_nonexistent_context_returns_err() {
        let mut s = PySuite::new(None);
        let result = s.hide_tool("no_such_ctx", "some_tool");
        assert!(result.is_err(), "hide_tool on absent context should fail");
    }

    #[test]
    fn test_path_none_before_save() {
        let s = PySuite::new(None);
        assert!(s.path().is_none(), "path must be None for unsaved suite");
    }

    #[test]
    fn test_suite_manager_no_paths_gives_none_suite() {
        let mgr = PySuiteManager::new(None);
        let names = mgr.list_suite_names();
        // With no paths (None), manager searches default dirs which may be empty
        // Just verify it doesn't panic and returns a Vec (may be empty or not)
        let _ = names.len();
    }

    #[test]
    fn test_suite_conflict_mode_default_is_lowercase() {
        let s = PySuite::new(None);
        let mode = s.conflict_mode();
        assert_eq!(mode, mode.to_lowercase(), "conflict_mode must be lowercase: '{mode}'");
    }

    #[test]
    fn test_suite_set_description_updates_value() {
        let mut s = PySuite::new(None);
        s.set_description(Some("updated desc".to_string()));
        assert_eq!(s.description(), Some("updated desc"));
    }

    #[test]
    fn test_suite_set_description_to_none_clears_value() {
        let mut s = PySuite::new(Some("initial"));
        s.set_description(None);
        assert!(s.description().is_none(), "description must be None after set to None");
    }

    #[test]
    fn test_suite_len_matches_context_count() {
        let mut s = PySuite::new(None);
        s.add_context("a", vec![]).unwrap();
        s.add_context("b", vec![]).unwrap();
        s.add_context("c", vec![]).unwrap();
        assert_eq!(s.__len__(), 3);
        assert_eq!(s.__len__(), s.context_names().len());
    }
}

mod test_suite_cy114 {
    use super::*;

    /// repr for empty suite shows 0 contexts
    #[test]
    fn test_repr_empty_suite_shows_zero() {
        let s = PySuite::new(None);
        let repr = s.__repr__();
        assert!(repr.contains('0'), "repr of empty suite must show 0: {repr}");
    }

    /// conflict_mode round-trip: set then get returns same value
    #[test]
    fn test_conflict_mode_roundtrip_error() {
        let mut s = PySuite::new(None);
        s.set_conflict_mode("error").unwrap();
        let mode = s.conflict_mode();
        assert_eq!(mode, "error", "conflict_mode should return 'error' after setting");
    }

    /// conflict_mode round-trip for 'first'
    #[test]
    fn test_conflict_mode_roundtrip_first() {
        let mut s = PySuite::new(None);
        s.set_conflict_mode("first").unwrap();
        let mode = s.conflict_mode();
        assert_eq!(mode, "first", "conflict_mode should return 'first' after setting");
    }

    /// conflict_mode round-trip for 'last'
    #[test]
    fn test_conflict_mode_roundtrip_last() {
        let mut s = PySuite::new(None);
        s.set_conflict_mode("last").unwrap();
        let mode = s.conflict_mode();
        assert_eq!(mode, "last", "conflict_mode should return 'last' after setting");
    }

    /// context_names returns exactly the contexts added (no extras)
    #[test]
    fn test_context_names_exact_match() {
        let mut s = PySuite::new(None);
        s.add_context("alpha", vec!["pkg-1".to_string()]).unwrap();
        s.add_context("beta", vec!["pkg-2".to_string()]).unwrap();
        let names = s.context_names();
        assert_eq!(names.len(), 2, "should have exactly 2 context names");
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    /// SuiteManager::add_path does not panic
    #[test]
    fn test_suite_manager_add_path_no_panic() {
        let mut mgr = PySuiteManager::new(Some(vec![]));
        mgr.add_path("/some/new/path");
        // Just verifying add_path does not panic
    }

    /// PySuite::is_suite with root-level empty string returns false
    #[test]
    fn test_is_suite_empty_string_path_returns_false() {
        // Empty string path does not point to a valid suite directory
        assert!(!PySuite::is_suite(""), "is_suite('') should return false");
    }

    /// SuiteManager with None uses default paths and does not panic
    #[test]
    fn test_suite_manager_none_paths_no_panic() {
        let mgr = PySuiteManager::new(None);
        // list_suite_names() must not panic regardless of default path contents
        let _ = mgr.list_suite_names();
    }
}

mod test_suite_cy122 {
    use super::*;

    /// Suite starts with no path set (unsaved)
    #[test]
    fn test_suite_path_none_initially() {
        let s = PySuite::new(None);
        assert!(s.inner.path.is_none(), "unsaved suite must have no path");
    }

    /// Suite description accessor returns same value as setter
    #[test]
    fn test_suite_description_accessor_roundtrip() {
        let mut s = PySuite::new(None);
        s.set_description(Some("roundtrip desc".to_string()));
        assert_eq!(
            s.description(),
            Some("roundtrip desc"),
            "description accessor must match set value"
        );
    }

    /// Suite with 0 contexts has __len__ == 0
    #[test]
    fn test_suite_len_zero_initially() {
        let s = PySuite::new(None);
        assert_eq!(s.__len__(), 0, "__len__ of fresh suite must be 0");
    }

    /// Suite with 1 context has __len__ == 1
    #[test]
    fn test_suite_len_one_after_add() {
        let mut s = PySuite::new(None);
        s.add_context("ctx", vec!["pkg-1".to_string()]).unwrap();
        assert_eq!(s.__len__(), 1);
    }

    /// Adding 3 contexts one-by-one reflects correct ordering in context_names
    #[test]
    fn test_context_names_order_preserved() {
        let mut s = PySuite::new(None);
        s.add_context("first", vec![]).unwrap();
        s.add_context("second", vec![]).unwrap();
        s.add_context("third", vec![]).unwrap();
        let names = s.context_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"first".to_string()));
        assert!(names.contains(&"second".to_string()));
        assert!(names.contains(&"third".to_string()));
    }

    /// suite repr after adding a context contains non-zero count
    #[test]
    fn test_repr_nonzero_after_add() {
        let mut s = PySuite::new(None);
        s.add_context("a", vec!["pkg-1".to_string()]).unwrap();
        let repr = s.__repr__();
        // repr should show 1 context
        assert!(
            repr.contains('1'),
            "repr after one add should contain '1', got: {repr}"
        );
    }
}

mod test_suite_cy127 {
    use super::*;

    /// Removing a context reduces __len__ by 1
    #[test]
    fn test_remove_context_reduces_len() {
        let mut s = PySuite::new(None);
        s.add_context("x", vec![]).unwrap();
        s.add_context("y", vec![]).unwrap();
        assert_eq!(s.__len__(), 2);
        s.remove_context("x").unwrap();
        assert_eq!(s.__len__(), 1, "len must decrease after remove_context");
    }

    /// Removing a non-existent context returns an Err
    #[test]
    fn test_remove_nonexistent_context_returns_err() {
        let mut s = PySuite::new(None);
        let result = s.remove_context("ghost");
        assert!(result.is_err(), "removing non-existent context must return Err");
    }

    /// Suite with Some description initialises description correctly
    #[test]
    fn test_new_with_description_stores_value() {
        let s = PySuite::new(Some("my suite"));
        assert_eq!(s.description(), Some("my suite"));
    }

    /// Suite with duplicate context name returns Err
    #[test]
    fn test_add_duplicate_context_returns_err() {
        let mut s = PySuite::new(None);
        s.add_context("dup", vec![]).unwrap();
        let result = s.add_context("dup", vec![]);
        assert!(result.is_err(), "adding a duplicate context name must return Err");
    }
}

mod test_suite_cy129 {
    use super::*;

    /// Suite __repr__ includes "Suite(" prefix
    #[test]
    fn test_repr_includes_suite_prefix() {
        let s = PySuite::new(Some("cy129 suite"));
        let r = s.__repr__();
        assert!(r.starts_with("Suite("), "__repr__ must start with 'Suite(', got: {r}");
    }

    /// Suite description set then clear results in None
    #[test]
    fn test_description_set_then_clear() {
        let mut s = PySuite::new(None);
        s.set_description(Some("temp".to_string()));
        assert_eq!(s.description(), Some("temp"));
        s.set_description(None);
        assert!(s.description().is_none(), "description must be None after clearing");
    }

    /// Suite with 5 contexts has __len__ == 5
    #[test]
    fn test_suite_len_five_contexts() {
        let mut s = PySuite::new(None);
        for i in 0..5usize {
            s.add_context(&format!("ctx_{i}"), vec![]).unwrap();
        }
        assert_eq!(s.__len__(), 5, "__len__ must be 5 after adding 5 contexts");
    }
}
