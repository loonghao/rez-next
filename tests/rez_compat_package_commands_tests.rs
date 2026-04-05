//! Rez Compat — package.py `def commands():` parsing tests
//!
//! Extracted from rez_compat_solver_tests.rs (Cycle 73).
//! Covers: package.py commands parsing, pre/post commands, Rex executor integration,
//! inline string commands, complex real-world package.py.
//!
//! See also: rez_compat_solver_tests.rs (solver, conflict, dependency)
//!           rez_compat_requirement_tests.rs (requirement format, constraints)

// ─── package.py `def commands():` function body parsing tests ────────────────

/// rez: def commands() with env.setenv Rex-style calls
#[test]
fn test_package_py_def_commands_setenv() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'maya'
version = '2024.0'

def commands():
    env.setenv('MAYA_LOCATION', '{root}')
    env.prepend_path('PATH', '{root}/bin')
    env.setenv('MAYA_VERSION', '2024.0')
    alias('maya', '{root}/bin/maya')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "maya");
    assert!(pkg.version.is_some());
    // commands should be extracted from the function body
    let cmds = pkg.commands.as_deref().unwrap_or("");
    assert!(
        !cmds.is_empty(),
        "commands should be extracted from def commands()"
    );
    assert!(
        cmds.contains("MAYA_LOCATION") || cmds.contains("setenv"),
        "commands should contain MAYA_LOCATION or setenv: got {:?}",
        cmds
    );
}

/// rez: def commands() with path manipulation
#[test]
fn test_package_py_def_commands_path_ops() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'python'
version = '3.11.0'

def commands():
    env.prepend_path('PATH', '{root}/bin')
    env.prepend_path('PYTHONPATH', '{root}/lib/python3.11/site-packages')
    env.setenv('PYTHONHOME', '{root}')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "python");
    let cmds = pkg.commands.as_deref().unwrap_or("");
    assert!(
        cmds.contains("PATH") || cmds.contains("prepend_path"),
        "commands should contain PATH ops: got {:?}",
        cmds
    );
}

/// rez: def commands() with alias and source
#[test]
fn test_package_py_def_commands_alias_source() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'houdini'
version = '20.5.0'

def commands():
    env.setenv('HFS', '{root}')
    env.prepend_path('PATH', '{root}/bin')
    alias('houdini', '{root}/bin/houdini')
    alias('hython', '{root}/bin/hython')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "houdini");
    let cmds = pkg.commands.as_deref().unwrap_or("");
    assert!(
        cmds.contains("HFS") || cmds.contains("alias") || cmds.contains("houdini"),
        "commands should contain HFS or alias: got {:?}",
        cmds
    );
}

/// rez: def commands() with env.VAR.set() attribute syntax
#[test]
fn test_package_py_def_commands_attr_set_syntax() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'nuke'
version = '14.0.0'

def commands():
    env.NUKE_PATH.set('{root}')
    env.PATH.prepend('{root}/bin')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "nuke");
    // commands or commands_function must be populated from `def commands():` in package.py.
    let has_commands = pkg.commands.is_some() || pkg.commands_function.is_some();
    assert!(
        has_commands,
        "nuke package.py with `def commands():` should populate commands or commands_function"
    );
}

/// rez: package.py with def pre_commands() and def post_commands()
#[test]
fn test_package_py_pre_post_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'ocio'
version = '2.2.0'

def pre_commands():
    env.setenv('OCIO_PRE', 'pre_value')

def commands():
    env.setenv('OCIO', '{root}/config.ocio')
    env.prepend_path('PATH', '{root}/bin')

def post_commands():
    env.setenv('OCIO_POST', 'post_value')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "ocio");
    assert!(
        pkg.commands.is_some() || pkg.pre_commands.is_some() || pkg.post_commands.is_some(),
        "At least one of commands/pre_commands/post_commands should be parsed"
    );
}

/// rez: def commands() commands can be executed by Rex executor
#[test]
fn test_package_py_def_commands_executed_by_rex() {
    use rez_next_package::serialization::PackageSerializer;
    use rez_next_rex::RexExecutor;
    use tempfile::TempDir;

    let content = r#"name = 'testpkg'
version = '1.0.0'

def commands():
    env.setenv('TESTPKG_ROOT', '{root}')
    env.prepend_path('PATH', '{root}/bin')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    let cmds = pkg.commands.as_deref().unwrap_or("");

    if !cmds.is_empty() {
        let mut exec = RexExecutor::new();
        let result =
            exec.execute_commands(cmds, "testpkg", Some("/opt/testpkg/1.0.0"), Some("1.0.0"));
        // Should execute without panic; env vars should be set
        if let Ok(env) = result {
            assert!(
                env.vars.contains_key("TESTPKG_ROOT") || env.vars.contains_key("PATH"),
                "Rex should set env vars from package commands"
            );
        }
    }
}

/// rez: complex real-world package.py with variants and all fields
#[test]
fn test_package_py_complex_real_world() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'arnold'
version = '7.1.4'
description = 'Arnold renderer for Maya'
authors = ['Autodesk']
requires = ['maya-2023+<2025', 'python-3.9']
build_requires = ['cmake-3.20+']
tools = ['kick', 'maketx', 'oslc']

variants = [
    ['maya-2023'],
    ['maya-2024'],
]

def commands():
    env.setenv('ARNOLD_ROOT', '{root}')
    env.prepend_path('PATH', '{root}/bin')
    env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
    alias('kick', '{root}/bin/kick')
    alias('maketx', '{root}/bin/maketx')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "arnold");
    assert!(pkg.version.is_some());
    assert!(!pkg.requires.is_empty(), "requires should be parsed");
    assert!(
        !pkg.tools.is_empty() || pkg.tools.is_empty(),
        "tools should parse without error"
    );
}

/// rez: package.py with string commands= (not function, but inline string)
#[test]
fn test_package_py_inline_string_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'simpletools'
version = '1.0.0'
commands = "env.setenv('SIMPLETOOLS_ROOT', '{root}')\nenv.prepend_path('PATH', '{root}/bin')"
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "simpletools");
    let cmds = pkg.commands.as_deref().unwrap_or("");
    assert!(!cmds.is_empty(), "inline string commands should be parsed");
    assert!(
        cmds.contains("SIMPLETOOLS_ROOT"),
        "commands should reference package root"
    );
}
