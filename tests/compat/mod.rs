//! Rez Compatibility Integration Tests (modular)
//!
//! Split from monolithic rez_compat_tests.rs (was 6925 lines, now modular sub-files).
//! Each sub-module contains tests for a specific domain area.

// Version & Package basics
#[path = "version_compat.rs"]
mod version_compat;
#[path = "package_parsing.rs"]
mod package_parsing;

// Rex & Suite
#[path = "rex_execution.rs"]
mod rex_execution;
#[path = "suite_management.rs"]
mod suite_management;

// Config & E2E
#[path = "config_and_e2e.rs"]
mod config_and_e2e;

// Core behavior
#[path = "official_behavior.rs"]
mod official_behavior;
#[path = "conflict_detection.rs"]
mod conflict_detection;
#[path = "commands_parsing.rs"]
mod commands_parsing;
#[path = "requirement_format.rs"]
mod requirement_format;

// Phase 2+ additions
#[path = "phase2_new.rs"]
mod phase2_new;
#[path = "pip_conversion.rs"]
mod pip_conversion;
#[path = "solver_conflict.rs"]
mod solver_conflict;
#[path = "complex_requirement.rs"]
mod complex_requirement;

// Module-level compat: source, data, context
#[path = "source_module.rs"]
mod source_module;
#[path = "data_module.rs"]
mod data_module;
#[path = "context_edge.rs"]
mod context_edge;
#[path = "rex_dsl_edge.rs"]
mod rex_dsl_edge;
#[path = "ctx_serialization.rs"]
mod ctx_serialization;
#[path = "forward_compat.rs"]
mod forward_compat;
#[path = "release_compat.rs"]
mod release_compat;
#[path = "extra_version_req.rs"]
mod extra_version_req;

// Dependency graph & circular
#[path = "circular_dep.rs"]
mod circular_dep;
#[path = "bind_compat.rs"]
mod bind_compat;
#[path = "requires_private.rs"]
mod requires_private;
#[path = "depgraph_conflict.rs"]
mod depgraph_conflict;

// CLI command compat
#[path = "search_compat.rs"]
mod search_compat;
#[path = "depends_reverse.rs"]
mod depends_reverse;
#[path = "complete_compat.rs"]
mod complete_compat;
#[path = "diff_compat.rs"]
mod diff_compat;
#[path = "status_compat.rs"]
mod status_compat;

// Solver internals
#[path = "solver_config.rs"]
mod solver_config;
#[path = "depends_semantics.rs"]
mod depends_semantics;
#[path = "solver_boundary_a.rs"]
mod solver_boundary_a;
#[path = "context_compat.rs"]
mod context_compat;
#[path = "solver_boundary_b.rs"]
mod solver_boundary_b;

// Validation & advanced types
#[path = "package_validate.rs"]
mod package_validate;
#[path = "verrange_advanced.rs"]
mod verrange_advanced;
#[path = "rex_dsl_advanced.rs"]
mod rex_dsl_advanced;
#[path = "exception_tests.rs"]
mod exception_tests;
#[path = "version_advanced.rs"]
mod version_advanced;
#[path = "rex_dsl_complete.rs"]
mod rex_dsl_complete;
#[path = "pkg_validation.rs"]
mod pkg_validation;
#[path = "suite_integration.rs"]
mod suite_integration;
#[path = "solver_topology.rs"]
mod solver_topology;

// Roundtrip & boundary
#[path = "ctx_roundtrip.rs"]
mod ctx_roundtrip;
#[path = "version_boundary.rs"]
mod version_boundary;
#[path = "rex_boundary.rs"]
mod rex_boundary;
#[path = "sourcemode.rs"]
mod sourcemode;
#[path = "ctx_tools_compat.rs"]
mod ctx_tools_compat;
#[path = "solver_weak_ver.rs"]
mod solver_weak_ver;

// Later-phase additions (262-308+)
#[path = "version_boundary_262.rs"]
mod version_boundary_262;
#[path = "pkg_validate_271.rs"]
mod pkg_validate_271;
#[path = "rex_edge_276.rs"]
mod rex_edge_276;
#[path = "pkg_commands.rs"]
mod pkg_commands;
#[path = "ctx_activation_e2e.rs"]
mod ctx_activation_e2e;
#[path = "solver_weak_dep.rs"]
mod solver_weak_dep;
#[path = "pkg_serializer_cmds.rs"]
mod pkg_serializer_cmds;

// Phase 136-143 + final config/diff
#[path = "phase136_143.rs"]
mod phase136_143;
#[path = "rez_config_final.rs"]
mod rez_config_final;
#[path = "diff_final.rs"]
mod diff_final;
