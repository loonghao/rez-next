//! Tests for bundle/unbundle functions — split into category files (Cycle 181).
//! See bundle_functions_bundle_tests.rs, bundle_functions_unbundle_tests.rs,
//! bundle_functions_list_tests.rs for the actual test modules.

#[path = "bundle_functions_bundle_tests.rs"]
mod test_bundle_context;

#[path = "bundle_functions_unbundle_tests.rs"]
mod test_unbundle_context;

#[path = "bundle_functions_list_tests.rs"]
mod test_list_bundles;
