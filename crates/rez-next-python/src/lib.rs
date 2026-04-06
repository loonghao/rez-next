//! Python bindings for rez-next
//!
//! Provides a drop-in replacement for the original Rez Python API.
//! Usage: `import rez_next as rez` — all rez APIs work identically.

use pyo3::prelude::*;

// ── Shared infrastructure ─────────────────────────────────────────────────────
pub(crate) mod runtime;

// ── Domain-specific binding modules ──────────────────────────────────────────
mod bind_bindings;
mod completion_bindings;
mod config_bindings;
mod context_bindings;
mod data_bindings;
mod depends_bindings;
mod diff_bindings;
mod env_bindings;
mod exceptions_bindings;
mod forward_bindings;
mod package_bindings;
mod pip_bindings;
mod plugins_bindings;
mod release_bindings;
mod repository_bindings;
mod search_bindings;
mod shell_bindings;
mod solver_bindings;
mod source_bindings;
mod status_bindings;
mod suite_bindings;
mod system_bindings;
mod version_bindings;

// ── Top-level function modules ────────────────────────────────────────────────
mod build_functions;
mod bundle_functions;
mod cli_functions;
mod package_functions;
mod rex_functions;
mod selftest_functions;

use bind_bindings::{PyBindManager, PyBindResult};
use config_bindings::PyConfig;
use context_bindings::PyResolvedContext;
use data_bindings::PyRezData;
use env_bindings::{PyPackageFamily, PyRezEnv};
use forward_bindings::PyRezForward;
use package_bindings::{PyPackage, PyPackageRequirement};
use pip_bindings::PyPipPackage;
use plugins_bindings::{PyPlugin, PyPluginType, PyRezPluginManager};
use release_bindings::{PyReleaseManager, PyReleaseResult};
use repository_bindings::PyRepositoryManager;
use search_bindings::PySearchResult;
use shell_bindings::PyShell;
use solver_bindings::PySolver;
use source_bindings::PySourceManager;
use suite_bindings::{PySuite, PySuiteManager};
use system_bindings::PySystem;
use version_bindings::{PyVersion, PyVersionRange};

// Re-export top-level functions for use in submodule registration below
use build_functions::{build_package, get_build_system};
use bundle_functions::{bundle_context, list_bundles, unbundle_context};
use cli_functions::{cli_main, cli_run};
use package_functions::{
    copy_package, get_latest_package, get_package, get_package_family_names, iter_packages,
    move_package, remove_package, resolve_packages, walk_packages,
};
use rex_functions::rex_interpret;
use selftest_functions::selftest;

/// Register a submodule and insert it into `sys.modules` so that dotted-path imports work.
///
/// pyo3's `add_submodule()` adds the module as an attribute but does NOT register it in
/// `sys.modules`. Without this registration, `from rez_next._native.<sub> import *` raises
/// `ModuleNotFoundError` even though the attribute exists on the parent module.
fn register_submodule(
    parent: &Bound<'_, PyModule>,
    name: &str,
    submod: &Bound<'_, PyModule>,
) -> PyResult<()> {
    parent.add_submodule(submod)?;

    // Build the full dotted name: e.g. "rez_next._native.config"
    let parent_name = parent.name()?;
    let full_name = format!("{}.{}", parent_name, name);

    // Insert into sys.modules
    let sys = pyo3::types::PyModule::import(parent.py(), "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(full_name.as_str(), submod)?;

    Ok(())
}

/// Main Python module `rez_next._native` — native extension backing the Python layer
#[pymodule(name = "_native")]
fn rez_next_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // ── Core types ────────────────────────────────────────────────────────────
    m.add_class::<PyVersion>()?;
    m.add_class::<PyVersionRange>()?;
    m.add_class::<PyPackage>()?;
    m.add_class::<PyPackageRequirement>()?;
    m.add_class::<PySolver>()?;
    m.add_class::<PyResolvedContext>()?;
    m.add_class::<PyConfig>()?;
    m.add_class::<PyRepositoryManager>()?;
    m.add_class::<PySuite>()?;
    m.add_class::<PySuiteManager>()?;
    m.add_class::<PySystem>()?;
    m.add_class::<PyShell>()?;
    m.add_class::<PyPipPackage>()?;
    m.add_class::<PyPlugin>()?;
    m.add_class::<PyPluginType>()?;
    m.add_class::<PyRezPluginManager>()?;
    m.add_class::<PyRezEnv>()?;
    m.add_class::<PyPackageFamily>()?;
    m.add_class::<PyRezForward>()?;
    m.add_class::<PyReleaseManager>()?;
    m.add_class::<PyReleaseResult>()?;
    m.add_class::<PySourceManager>()?;
    m.add_class::<PyRezData>()?;
    m.add_class::<PyBindManager>()?;
    m.add_class::<PyBindResult>()?;

    // ── Top-level convenience functions ───────────────────────────────────────
    m.add_function(wrap_pyfunction!(get_latest_package, m)?)?;
    m.add_function(wrap_pyfunction!(get_package, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_packages, m)?)?;
    m.add_function(wrap_pyfunction!(iter_packages, m)?)?;
    m.add_function(wrap_pyfunction!(get_package_family_names, m)?)?;
    m.add_function(wrap_pyfunction!(copy_package, m)?)?;
    m.add_function(wrap_pyfunction!(move_package, m)?)?;
    m.add_function(wrap_pyfunction!(remove_package, m)?)?;
    m.add_function(wrap_pyfunction!(walk_packages, m)?)?;
    m.add_function(wrap_pyfunction!(selftest, m)?)?;
    m.add_function(wrap_pyfunction!(build_package, m)?)?;
    m.add_function(wrap_pyfunction!(bundle_context, m)?)?;
    m.add_function(wrap_pyfunction!(pip_bindings::pip_install, m)?)?;
    m.add_function(wrap_pyfunction!(plugins_bindings::get_plugin_manager, m)?)?;
    m.add_function(wrap_pyfunction!(env_bindings::create_env, m)?)?;
    m.add_function(wrap_pyfunction!(env_bindings::get_activation_script, m)?)?;
    m.add_function(wrap_pyfunction!(forward_bindings::resolve_forward_tool, m)?)?;
    m.add_function(wrap_pyfunction!(
        forward_bindings::generate_forward_script,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(release_bindings::release_package, m)?)?;
    m.add_function(wrap_pyfunction!(source_bindings::write_source_script, m)?)?;
    m.add_function(wrap_pyfunction!(source_bindings::get_source_script, m)?)?;
    m.add_function(wrap_pyfunction!(source_bindings::detect_shell, m)?)?;
    m.add_function(wrap_pyfunction!(source_bindings::resolve_source_mode, m)?)?;
    m.add_function(wrap_pyfunction!(search_bindings::search_packages, m)?)?;
    m.add_function(wrap_pyfunction!(search_bindings::search_package_names, m)?)?;
    m.add_function(wrap_pyfunction!(
        completion_bindings::get_completion_script,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(diff_bindings::diff_contexts, m)?)?;
    m.add_function(wrap_pyfunction!(diff_bindings::diff_context_files, m)?)?;
    m.add_function(wrap_pyfunction!(diff_bindings::format_diff, m)?)?;
    m.add_function(wrap_pyfunction!(status_bindings::is_in_rez_context, m)?)?;
    m.add_function(wrap_pyfunction!(status_bindings::get_current_status, m)?)?;
    m.add_function(wrap_pyfunction!(
        depends_bindings::get_reverse_dependencies,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(depends_bindings::get_dependants, m)?)?;
    m.add_function(wrap_pyfunction!(depends_bindings::print_depends, m)?)?;
    m.add_function(wrap_pyfunction!(bind_bindings::bind_tool, m)?)?;
    m.add_function(wrap_pyfunction!(bind_bindings::list_binders, m)?)?;

    // ── Module metadata & singletons ──────────────────────────────────────────
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "rez-next contributors")?;
    m.add("config", PyConfig::new())?;
    m.add("system", PySystem::new())?;

    // ── Submodule: rez.exceptions ─────────────────────────────────────────────
    let exceptions = PyModule::new(m.py(), "exceptions")?;
    exceptions_bindings::register_all_exceptions(&exceptions)?;
    register_submodule(m, "exceptions", &exceptions)?;
    m.add(
        "RezError",
        m.py().get_type::<exceptions_bindings::RezError>(),
    )?;
    m.add(
        "PackageNotFound",
        m.py().get_type::<exceptions_bindings::PackageNotFound>(),
    )?;
    m.add(
        "ResolveError",
        m.py().get_type::<exceptions_bindings::ResolveError>(),
    )?;
    m.add(
        "RezBuildError",
        m.py().get_type::<exceptions_bindings::RezBuildError>(),
    )?;
    m.add(
        "ConfigurationError",
        m.py().get_type::<exceptions_bindings::ConfigurationError>(),
    )?;

    // ── Submodule: rez.packages_ ──────────────────────────────────────────────
    let packages_ = PyModule::new(m.py(), "packages_")?;
    packages_.add_function(wrap_pyfunction!(iter_packages, &packages_)?)?;
    packages_.add_function(wrap_pyfunction!(get_latest_package, &packages_)?)?;
    packages_.add_function(wrap_pyfunction!(get_package, &packages_)?)?;
    packages_.add_function(wrap_pyfunction!(get_package_family_names, &packages_)?)?;
    packages_.add_function(wrap_pyfunction!(walk_packages, &packages_)?)?;
    packages_.add_function(wrap_pyfunction!(copy_package, &packages_)?)?;
    packages_.add_function(wrap_pyfunction!(move_package, &packages_)?)?;
    packages_.add_function(wrap_pyfunction!(remove_package, &packages_)?)?;
    register_submodule(m, "packages_", &packages_)?;

    // ── Submodule: rez.resolved_context ───────────────────────────────────────
    let resolved_context = PyModule::new(m.py(), "resolved_context")?;
    resolved_context.add_class::<PyResolvedContext>()?;
    register_submodule(m, "resolved_context", &resolved_context)?;

    // ── Submodule: rez.suite ──────────────────────────────────────────────────
    let suite_mod = PyModule::new(m.py(), "suite")?;
    suite_mod.add_class::<PySuite>()?;
    suite_mod.add_class::<PySuiteManager>()?;
    register_submodule(m, "suite", &suite_mod)?;

    // ── Submodule: rez.config ─────────────────────────────────────────────────
    let config_mod = PyModule::new(m.py(), "config")?;
    config_mod.add_class::<PyConfig>()?;
    config_mod.add("config", PyConfig::new())?;
    register_submodule(m, "config", &config_mod)?;

    // ── Submodule: rez.system ─────────────────────────────────────────────────
    let system_mod = PyModule::new(m.py(), "system")?;
    system_mod.add_class::<PySystem>()?;
    system_mod.add("system", PySystem::new())?;
    system_mod.add_function(wrap_pyfunction!(system_bindings::get_system, &system_mod)?)?;
    register_submodule(m, "system", &system_mod)?;

    // ── Submodule: rez.vendor.version ─────────────────────────────────────────
    let vendor = PyModule::new(m.py(), "vendor")?;
    let version_mod = PyModule::new(m.py(), "version")?;
    version_mod.add_class::<PyVersion>()?;
    version_mod.add_class::<PyVersionRange>()?;
    vendor.add_submodule(&version_mod)?;
    {
        let sys = pyo3::types::PyModule::import(m.py(), "sys")?;
        let modules = sys.getattr("modules")?;
        modules.set_item("rez_next._native.vendor", &vendor)?;
        modules.set_item("rez_next._native.vendor.version", &version_mod)?;
    }
    register_submodule(m, "vendor", &vendor)?;

    // ── Submodule: rez.build_ ─────────────────────────────────────────────────
    let build_mod = PyModule::new(m.py(), "build_")?;
    build_mod.add_function(wrap_pyfunction!(build_package, &build_mod)?)?;
    build_mod.add_function(wrap_pyfunction!(get_build_system, &build_mod)?)?;

    // ── Submodule: rez.rex ────────────────────────────────────────────────────
    let rex_mod = PyModule::new(m.py(), "rex")?;
    rex_mod.add_function(wrap_pyfunction!(rex_interpret, &rex_mod)?)?;
    register_submodule(m, "rex", &rex_mod)?;

    // ── Submodule: rez.shell ──────────────────────────────────────────────────
    let shell_mod = PyModule::new(m.py(), "shell")?;
    shell_mod.add_class::<PyShell>()?;
    shell_mod.add_function(wrap_pyfunction!(
        shell_bindings::create_shell_script,
        &shell_mod
    )?)?;
    shell_mod.add_function(wrap_pyfunction!(
        shell_bindings::get_available_shells,
        &shell_mod
    )?)?;
    shell_mod.add_function(wrap_pyfunction!(
        shell_bindings::get_current_shell,
        &shell_mod
    )?)?;
    register_submodule(m, "shell", &shell_mod)?;

    // ── Submodule: rez.bundles ────────────────────────────────────────────────
    let bundles_mod = PyModule::new(m.py(), "bundles")?;
    bundles_mod.add_function(wrap_pyfunction!(bundle_context, &bundles_mod)?)?;
    bundles_mod.add_function(wrap_pyfunction!(unbundle_context, &bundles_mod)?)?;
    bundles_mod.add_function(wrap_pyfunction!(list_bundles, &bundles_mod)?)?;
    register_submodule(m, "bundles", &bundles_mod)?;

    // ── Submodule: rez.cli ────────────────────────────────────────────────────
    let cli_mod = PyModule::new(m.py(), "cli")?;
    cli_mod.add_function(wrap_pyfunction!(cli_run, &cli_mod)?)?;
    cli_mod.add_function(wrap_pyfunction!(cli_main, &cli_mod)?)?;
    register_submodule(m, "cli", &cli_mod)?;

    // ── Submodule: rez.utils.resources ───────────────────────────────────────
    let utils_mod = PyModule::new(m.py(), "utils")?;
    let resources_mod = PyModule::new(m.py(), "resources")?;
    resources_mod.add_function(wrap_pyfunction!(get_resource_string, &resources_mod)?)?;
    utils_mod.add_submodule(&resources_mod)?;
    {
        let sys = pyo3::types::PyModule::import(m.py(), "sys")?;
        let modules = sys.getattr("modules")?;
        modules.set_item("rez_next._native.utils", &utils_mod)?;
        modules.set_item("rez_next._native.utils.resources", &resources_mod)?;
    }
    register_submodule(m, "utils", &utils_mod)?;

    // ── Submodule: rez.pip ────────────────────────────────────────────────────
    let pip_mod = PyModule::new(m.py(), "pip")?;
    pip_mod.add_class::<PyPipPackage>()?;
    pip_mod.add_function(wrap_pyfunction!(
        pip_bindings::normalize_package_name,
        &pip_mod
    )?)?;
    pip_mod.add_function(wrap_pyfunction!(
        pip_bindings::pip_version_to_rez,
        &pip_mod
    )?)?;
    pip_mod.add_function(wrap_pyfunction!(pip_bindings::pip_install, &pip_mod)?)?;
    pip_mod.add_function(wrap_pyfunction!(
        pip_bindings::convert_pip_to_rez,
        &pip_mod
    )?)?;
    pip_mod.add_function(wrap_pyfunction!(
        pip_bindings::get_pip_dependencies,
        &pip_mod
    )?)?;
    pip_mod.add_function(wrap_pyfunction!(pip_bindings::write_pip_package, &pip_mod)?)?;
    register_submodule(m, "pip", &pip_mod)?;

    // ── Submodule: rez.plugins ────────────────────────────────────────────────
    let plugins_mod = PyModule::new(m.py(), "plugins")?;
    plugins_mod.add_class::<plugins_bindings::PyPluginType>()?;
    plugins_mod.add_class::<PyPlugin>()?;
    plugins_mod.add_class::<PyRezPluginManager>()?;
    plugins_mod.add_function(wrap_pyfunction!(
        plugins_bindings::get_plugin_manager,
        &plugins_mod
    )?)?;
    plugins_mod.add_function(wrap_pyfunction!(
        plugins_bindings::get_shell_types,
        &plugins_mod
    )?)?;
    plugins_mod.add_function(wrap_pyfunction!(
        plugins_bindings::get_build_system_types,
        &plugins_mod
    )?)?;
    plugins_mod.add_function(wrap_pyfunction!(
        plugins_bindings::is_shell_supported,
        &plugins_mod
    )?)?;
    plugins_mod.add("plugin_manager", plugins_bindings::get_plugin_manager())?;
    register_submodule(m, "plugins", &plugins_mod)?;

    // ── Submodule: rez.env ────────────────────────────────────────────────────
    let env_mod = PyModule::new(m.py(), "env")?;
    env_mod.add_class::<PyRezEnv>()?;
    env_mod.add_class::<PyPackageFamily>()?;
    env_mod.add_function(wrap_pyfunction!(env_bindings::create_env, &env_mod)?)?;
    env_mod.add_function(wrap_pyfunction!(
        env_bindings::get_activation_script,
        &env_mod
    )?)?;
    env_mod.add_function(wrap_pyfunction!(env_bindings::apply_env, &env_mod)?)?;
    register_submodule(m, "env", &env_mod)?;

    // ── Submodule: rez.packages ───────────────────────────────────────────────
    let packages_mod = PyModule::new(m.py(), "packages")?;
    packages_mod.add_class::<PyPackageFamily>()?;
    packages_mod.add_class::<PyPackage>()?;
    packages_mod.add_class::<PyPackageRequirement>()?;
    register_submodule(m, "packages", &packages_mod)?;

    // ── Submodule: rez.forward ────────────────────────────────────────────────
    let forward_mod = PyModule::new(m.py(), "forward")?;
    forward_mod.add_class::<PyRezForward>()?;
    forward_mod.add_function(wrap_pyfunction!(
        forward_bindings::resolve_forward_tool,
        &forward_mod
    )?)?;
    forward_mod.add_function(wrap_pyfunction!(
        forward_bindings::generate_forward_script,
        &forward_mod
    )?)?;
    register_submodule(m, "forward", &forward_mod)?;

    // ── Submodule: rez.release ────────────────────────────────────────────────
    let release_mod = PyModule::new(m.py(), "release")?;
    release_mod.add_class::<PyReleaseManager>()?;
    release_mod.add_class::<PyReleaseResult>()?;
    release_mod.add_function(wrap_pyfunction!(
        release_bindings::release_package,
        &release_mod
    )?)?;
    register_submodule(m, "release", &release_mod)?;

    // ── Submodule: rez.source ─────────────────────────────────────────────────
    let source_mod = PyModule::new(m.py(), "source")?;
    source_mod.add_class::<PySourceManager>()?;
    source_mod.add_function(wrap_pyfunction!(
        source_bindings::write_source_script,
        &source_mod
    )?)?;
    source_mod.add_function(wrap_pyfunction!(
        source_bindings::get_source_script,
        &source_mod
    )?)?;
    source_mod.add_function(wrap_pyfunction!(
        source_bindings::detect_shell,
        &source_mod
    )?)?;
    source_mod.add_function(wrap_pyfunction!(
        source_bindings::resolve_source_mode,
        &source_mod
    )?)?;
    register_submodule(m, "source", &source_mod)?;

    // ── Submodule: rez.data ───────────────────────────────────────────────────
    let data_mod = PyModule::new(m.py(), "data")?;
    data_mod.add_class::<PyRezData>()?;
    data_mod.add_function(wrap_pyfunction!(
        data_bindings::get_data_resource,
        &data_mod
    )?)?;
    data_mod.add_function(wrap_pyfunction!(
        data_bindings::list_data_resources,
        &data_mod
    )?)?;
    data_mod.add_function(wrap_pyfunction!(
        data_bindings::get_completion_script,
        &data_mod
    )?)?;
    data_mod.add("data", PyRezData::new())?;
    register_submodule(m, "data", &data_mod)?;

    // ── Submodule: rez.bind ───────────────────────────────────────────────────
    let bind_mod = PyModule::new(m.py(), "bind")?;
    bind_mod.add_class::<PyBindManager>()?;
    bind_mod.add_class::<PyBindResult>()?;
    bind_mod.add_function(wrap_pyfunction!(bind_bindings::bind_tool, &bind_mod)?)?;
    bind_mod.add_function(wrap_pyfunction!(bind_bindings::list_binders, &bind_mod)?)?;
    bind_mod.add_function(wrap_pyfunction!(bind_bindings::detect_version, &bind_mod)?)?;
    bind_mod.add_function(wrap_pyfunction!(bind_bindings::find_tool, &bind_mod)?)?;
    bind_mod.add_function(wrap_pyfunction!(bind_bindings::extract_version, &bind_mod)?)?;
    bind_mod.add("bind_manager", PyBindManager::new())?;
    register_submodule(m, "bind", &bind_mod)?;

    // ── Submodule: rez.search ─────────────────────────────────────────────────
    let search_mod = PyModule::new(m.py(), "search")?;
    search_mod.add_class::<PySearchResult>()?;
    search_mod.add_class::<search_bindings::PyPackageSearcher>()?;
    search_mod.add_function(wrap_pyfunction!(
        search_bindings::search_packages,
        &search_mod
    )?)?;
    search_mod.add_function(wrap_pyfunction!(
        search_bindings::search_package_names,
        &search_mod
    )?)?;
    search_mod.add_function(wrap_pyfunction!(
        search_bindings::search_latest_packages,
        &search_mod
    )?)?;
    register_submodule(m, "search", &search_mod)?;

    // ── Submodule: rez.complete ───────────────────────────────────────────────
    let complete_mod = PyModule::new(m.py(), "complete")?;
    complete_mod.add_function(wrap_pyfunction!(
        completion_bindings::get_completion_script,
        &complete_mod
    )?)?;
    complete_mod.add_function(wrap_pyfunction!(
        completion_bindings::print_completion_script,
        &complete_mod
    )?)?;
    complete_mod.add_function(wrap_pyfunction!(
        completion_bindings::supported_completion_shells,
        &complete_mod
    )?)?;
    complete_mod.add_function(wrap_pyfunction!(
        completion_bindings::get_completion_install_path,
        &complete_mod
    )?)?;
    register_submodule(m, "complete", &complete_mod)?;

    // ── Submodule: rez.diff ───────────────────────────────────────────────────
    let diff_mod = PyModule::new(m.py(), "diff")?;
    diff_mod.add_class::<diff_bindings::PyPackageDiff>()?;
    diff_mod.add_class::<diff_bindings::PyContextDiff>()?;
    diff_mod.add_function(wrap_pyfunction!(diff_bindings::diff_contexts, &diff_mod)?)?;
    diff_mod.add_function(wrap_pyfunction!(
        diff_bindings::diff_context_files,
        &diff_mod
    )?)?;
    diff_mod.add_function(wrap_pyfunction!(diff_bindings::format_diff, &diff_mod)?)?;
    register_submodule(m, "diff", &diff_mod)?;

    // ── Submodule: rez.status ─────────────────────────────────────────────────
    let status_mod = PyModule::new(m.py(), "status")?;
    status_mod.add_class::<status_bindings::PyRezStatus>()?;
    status_mod.add_function(wrap_pyfunction!(
        status_bindings::get_current_status,
        &status_mod
    )?)?;
    status_mod.add_function(wrap_pyfunction!(
        status_bindings::is_in_rez_context,
        &status_mod
    )?)?;
    status_mod.add_function(wrap_pyfunction!(
        status_bindings::get_context_file,
        &status_mod
    )?)?;
    status_mod.add_function(wrap_pyfunction!(
        status_bindings::get_resolved_package_names,
        &status_mod
    )?)?;
    status_mod.add_function(wrap_pyfunction!(
        status_bindings::get_rez_env_var,
        &status_mod
    )?)?;
    register_submodule(m, "status", &status_mod)?;

    // ── Submodule: rez.depends ────────────────────────────────────────────────
    let depends_mod = PyModule::new(m.py(), "depends")?;
    depends_mod.add_class::<depends_bindings::PyDependsEntry>()?;
    depends_mod.add_class::<depends_bindings::PyDependsResult>()?;
    depends_mod.add_function(wrap_pyfunction!(
        depends_bindings::get_reverse_dependencies,
        &depends_mod
    )?)?;
    depends_mod.add_function(wrap_pyfunction!(
        depends_bindings::get_dependants,
        &depends_mod
    )?)?;
    depends_mod.add_function(wrap_pyfunction!(
        depends_bindings::print_depends,
        &depends_mod
    )?)?;
    register_submodule(m, "depends", &depends_mod)?;

    Ok(())
}

/// Get a resource string from rez-next (e.g., version, config schema).
/// Equivalent to `rez.utils.resources.get_resource_string(name)`
#[pyfunction]
fn get_resource_string(name: &str) -> PyResult<String> {
    match name {
        "version" => Ok(env!("CARGO_PKG_VERSION").to_string()),
        "name" => Ok("rez_next".to_string()),
        "description" => {
            Ok("rez-next: A Rust implementation of the rez package manager".to_string())
        }
        _ => Err(pyo3::exceptions::PyKeyError::new_err(format!(
            "Unknown resource: '{}'",
            name
        ))),
    }
}
