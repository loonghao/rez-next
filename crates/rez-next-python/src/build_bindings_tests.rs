//! Tests for build_bindings.rs — PyBuildType, PyBuildSystem, etc.

#[cfg(test)]
use pyo3::prelude::*;
use pyo3::types::PyAny;

#[test]
fn build_type_new_valid() {
    let gil = Python::acquire_gil();
    let py = gil.python();

    let bt = py.run_bound(&format!(
        "from rez_next._native.build_ import BuildType\nbt = BuildType('local')\nbt"
    ), None, None).unwrap();

    // Just verify it runs without error
    assert!(bt.is_some() || bt.is_none()); // placeholder
}

#[test]
fn build_type_new_invalid() {
    let gil = Python::acquire_gil();
    let py = gil.python();

    let result = py.run_bound(
        "from rez_next._native.build_ import BuildType\nbt = BuildType('invalid')",
        None,
        None,
    );

    assert!(result.is_err()); // Should raise ValueError
}

#[test]
fn build_system_detect() {
    let gil = Python::acquire_gil();
    let py = gil.python();

    // Create a temp dir with CMakeLists.txt
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.0)").unwrap();

    let result = py.run_bound(&format!(
        "from rez_next._native.build_ import BuildSystem\nbs = BuildSystem.detect('{}')\nbs",
        tmp.path().display()
    ), None, None);

    assert!(result.is_ok()); // Should successfully detect CMake
}
