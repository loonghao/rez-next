use super::*;

#[test]
fn test_fish_completion_not_empty() {
    assert!(
        FISH_COMPLETE.len() > 10,
        "fish completion should have meaningful content"
    );
    assert!(FISH_COMPLETE.contains("rez-next"));
    assert!(FISH_COMPLETE.contains("complete -c rez-next"));
}

#[test]
fn test_resource_lookup_fish() {
    let content = match "completions/fish" {
        "completions/fish" => FISH_COMPLETE.to_string(),
        _ => panic!("not found"),
    };
    assert!(!content.is_empty());
    assert!(content.contains("fish completion"));
}

#[test]
fn test_bash_completion_not_empty() {
    assert!(
        BASH_COMPLETE.len() > 10,
        "bash completion should have meaningful content"
    );
    assert!(BASH_COMPLETE.contains("_rez_next"));
}

#[test]
fn test_zsh_completion_not_empty() {
    assert!(
        ZSH_COMPLETE.len() > 10,
        "zsh completion should have meaningful content"
    );
    assert!(ZSH_COMPLETE.contains("rez-next"));
}

#[test]
fn test_example_package_py_valid() {
    assert!(EXAMPLE_PACKAGE_PY.contains("name"));
    assert!(EXAMPLE_PACKAGE_PY.contains("version"));
    assert!(EXAMPLE_PACKAGE_PY.contains("requires"));
}

#[test]
fn test_default_rezconfig_valid() {
    assert!(DEFAULT_REZCONFIG.contains("packages_path"));
    assert!(DEFAULT_REZCONFIG.contains("local_packages_path"));
}

#[test]
fn test_list_resources_non_empty() {
    let data = PyRezData::new();
    let resources = data.list_resources();
    assert!(!resources.is_empty());
    assert!(resources.contains(&"completions/bash".to_string()));
    assert!(resources.contains(&"examples/package.py".to_string()));
}

#[test]
fn test_write_completion_to_file() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("rez-complete.bash");
    let content = BASH_COMPLETE;
    std::fs::write(&dest, content).unwrap();
    let written = std::fs::read_to_string(&dest).unwrap();
    assert!(written.contains("_rez_next"));
}

#[test]
fn test_resource_lookup_bash() {
    let content = match "completions/bash" {
        "completions/bash" => BASH_COMPLETE.to_string(),
        _ => panic!("not found"),
    };
    assert!(!content.is_empty());
}

#[test]
fn test_resource_lookup_example_package() {
    let content = EXAMPLE_PACKAGE_PY;
    assert!(content.contains("name = \"my_package\""));
}

#[test]
fn test_write_example_package_to_dir() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let dest_dir = tmp.path().to_str().unwrap();
    let pkg_path = std::path::PathBuf::from(dest_dir).join("package.py");
    std::fs::write(&pkg_path, EXAMPLE_PACKAGE_PY).unwrap();
    assert!(pkg_path.exists());
    let content = std::fs::read_to_string(&pkg_path).unwrap();
    assert!(content.contains("my_package"));
}

#[test]
fn test_rez_data_new_no_panic() {
    let _d = PyRezData::new();
}

#[test]
fn test_rez_data_repr() {
    let d = PyRezData::new();
    assert_eq!(d.__repr__(), "RezData()");
}

#[test]
fn test_rez_data_list_resources_contains_completions() {
    let d = PyRezData::new();
    let resources = d.list_resources();
    assert!(
        resources.iter().any(|r| r.starts_with("completions/")),
        "list_resources must include completions/*, got: {:?}",
        resources
    );
}

#[test]
fn test_rez_data_list_resources_count() {
    let d = PyRezData::new();
    let r = d.list_resources();
    assert_eq!(r.len(), 5, "expected 5 resources, got {}", r.len());
}

#[test]
fn test_rez_data_get_resource_bash_ok() {
    let d = PyRezData::new();
    let content = d
        .get_resource("completions/bash")
        .expect("completions/bash must be a known resource");
    assert!(content.contains("_rez_next"), "bash completion must define _rez_next: {content}");
}

#[test]
fn test_rez_data_get_resource_zsh_ok() {
    let d = PyRezData::new();
    let content = d
        .get_resource("completions/zsh")
        .expect("completions/zsh must be a known resource");
    assert!(content.contains("rez-next"), "zsh completion must mention rez-next: {content}");
}

#[test]
fn test_rez_data_get_resource_fish_ok() {
    let d = PyRezData::new();
    let content = d
        .get_resource("completions/fish")
        .expect("completions/fish must be a known resource");
    assert!(
        content.contains("complete -c rez-next"),
        "fish completion must have 'complete -c rez-next' directive: {content}"
    );
}

#[test]
fn test_rez_data_get_resource_example_package_ok() {
    let d = PyRezData::new();
    let content = d
        .get_resource("examples/package.py")
        .expect("examples/package.py must be a known resource");
    assert!(
        content.contains("name = \"my_package\""),
        "example package must define name = \"my_package\": {content}"
    );
}

#[test]
fn test_rez_data_get_resource_config_ok() {
    let d = PyRezData::new();
    let content = d
        .get_resource("config/rezconfig.py")
        .expect("config/rezconfig.py must be a known resource");
    assert!(content.contains("packages_path"), "rezconfig must define packages_path: {content}");
}

#[test]
fn test_rez_data_get_resource_unknown_errors() {
    let d = PyRezData::new();
    let r = d.get_resource("unknown/path.txt");
    assert!(r.is_err(), "unknown resource should return Err");
}

#[test]
fn test_rez_data_get_example_package() {
    let d = PyRezData::new();
    let content = d.get_example_package();
    assert!(content.contains("name"));
    assert!(content.contains("version"));
}

#[test]
fn test_rez_data_get_default_config() {
    let d = PyRezData::new();
    let content = d.get_default_config();
    assert!(content.contains("packages_path"));
    assert!(content.contains("local_packages_path"));
}

#[test]
fn test_list_data_resources_non_empty() {
    let resources = list_data_resources();
    assert!(!resources.is_empty());
}

#[test]
fn test_get_data_resource_bash() {
    let content = get_data_resource("completions/bash")
        .expect("completions/bash must be a known data resource");
    assert!(
        content.contains("_rez_next"),
        "bash completion data resource must define _rez_next: {content}"
    );
}

#[test]
fn test_get_data_resource_unknown_errors() {
    let r = get_data_resource("no/such/resource");
    assert!(r.is_err());
}

#[test]
fn test_write_completion_script_to_file() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("rez-complete.bash").to_str().unwrap().to_string();
    let d = PyRezData::new();
    let result = d.write_completion_script(&dest, Some("bash"));
    assert!(result.is_ok(), "write_completion_script should succeed: {:?}", result);
    let written = std::fs::read_to_string(&dest).unwrap();
    assert!(written.contains("_rez_next"));
}

#[test]
fn test_write_completion_script_unknown_shell_errors() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("bad.sh").to_str().unwrap().to_string();
    let d = PyRezData::new();
    let result = d.write_completion_script(&dest, Some("ksh"));
    assert!(result.is_err(), "unknown shell should return Err");
}

#[test]
fn test_bash_complete_contains_compgen() {
    assert!(BASH_COMPLETE.contains("compgen"), "bash completion must use compgen");
}

#[test]
fn test_zsh_complete_contains_compdef() {
    assert!(ZSH_COMPLETE.contains("#compdef"), "zsh must have #compdef: {ZSH_COMPLETE}");
}

#[test]
fn test_fish_complete_contains_complete_c() {
    assert!(FISH_COMPLETE.contains("complete -c rez-next"), "fish completion must have `complete -c rez-next`");
}

#[test]
fn test_example_package_py_has_commands_fn() {
    assert!(EXAMPLE_PACKAGE_PY.contains("def commands():"), "package.py must define commands()");
}

#[test]
fn test_default_rezconfig_has_local_packages_path() {
    assert!(DEFAULT_REZCONFIG.contains("local_packages_path"), "rezconfig must define local_packages_path");
    assert!(DEFAULT_REZCONFIG.contains("default_shell"), "rezconfig must define default_shell");
}

#[test]
fn test_rez_data_get_completion_script_bash_no_panic() {
    let d = PyRezData::new();
    let script = d
        .get_completion_script(Some("bash"))
        .expect("bash completion must not error");
    assert!(
        script.contains("_rez_next"),
        "bash completion must define _rez_next function: {script}"
    );
}

#[test]
fn test_rez_data_get_completion_script_none_defaults_to_bash() {
    let d = PyRezData::new();
    let r = d.get_completion_script(None);
    assert!(r.is_ok(), "None shell should default to bash: {:?}", r);
    let content = r.unwrap();
    assert!(content.contains("_rez_next"), "default completion should be bash");
}

#[test]
fn test_rez_data_get_completion_script_zsh() {
    let d = PyRezData::new();
    let content = d
        .get_completion_script(Some("zsh"))
        .expect("zsh completion should succeed");
    assert!(
        content.contains("#compdef"),
        "zsh completion script must contain #compdef directive: {content}"
    );
}

#[test]
fn test_rez_data_get_completion_script_fish() {
    let d = PyRezData::new();
    let content = d
        .get_completion_script(Some("fish"))
        .expect("fish completion should succeed");
    assert!(
        content.contains("complete -c"),
        "fish completion script must contain 'complete -c' directive: {content}"
    );
}

#[test]
fn test_rez_data_get_completion_script_unknown_shell_errs() {
    let d = PyRezData::new();
    let r = d.get_completion_script(Some("tcsh_unknown_shell_xyz"));
    assert!(
        r.is_err(),
        "get_completion_script with unknown shell must return Err, got: {:?}",
        r
    );
}

#[test]
fn test_rez_data_default_config_template_non_empty() {
    let d = PyRezData::new();
    let tmpl = d.get_default_config();
    assert!(!tmpl.is_empty(), "default config template must not be empty");
}

#[test]
fn test_rez_data_bash_completion_contains_subcommands() {
    let d = PyRezData::new();
    let content = d.get_completion_script(Some("bash")).unwrap();
    assert!(content.contains("env"), "bash completion: {content}");
    assert!(content.contains("search"), "bash completion: {content}");
}

#[test]
fn test_rez_data_example_package_str_non_empty() {
    let d = PyRezData::new();
    let example = d.get_example_package();
    assert!(!example.is_empty(), "example package definition must not be empty");
    assert!(
        example.contains("name") || example.contains("version"),
        "example package must contain 'name' or 'version': {example}"
    );
}

#[test]
fn test_rez_data_new_is_deterministic() {
    let d1 = PyRezData::new();
    let d2 = PyRezData::new();
    let s1 = d1.get_completion_script(Some("bash")).unwrap();
    let s2 = d2.get_completion_script(Some("bash")).unwrap();
    assert_eq!(s1, s2, "PyRezData instances must produce identical outputs");
}

#[test]
fn test_bash_complete_contains_env_subcommand() {
    assert!(
        BASH_COMPLETE.contains("env"),
        "bash completion must include 'env' subcommand"
    );
}

#[test]
fn test_fish_complete_contains_search_subcommand() {
    assert!(
        FISH_COMPLETE.contains("search"),
        "fish completion must include 'search' subcommand"
    );
}

#[test]
fn test_zsh_complete_contains_solve_subcommand() {
    assert!(
        ZSH_COMPLETE.contains("solve"),
        "zsh completion must include 'solve' subcommand: {ZSH_COMPLETE}"
    );
}

#[test]
fn test_example_package_py_has_author_or_description() {
    let has_author = EXAMPLE_PACKAGE_PY.contains("authors") || EXAMPLE_PACKAGE_PY.contains("description");
    assert!(has_author, "example package.py must contain 'authors' or 'description' field");
}

#[test]
fn test_list_data_resources_includes_zsh() {
    let resources = list_data_resources();
    assert!(
        resources.contains(&"completions/zsh".to_string()),
        "list_data_resources must include completions/zsh: {:?}",
        resources
    );
}

#[test]
fn test_get_data_resource_zsh_contains_rez() {
    let r = get_data_resource("completions/zsh");
    assert!(r.is_ok(), "completions/zsh resource must be accessible");
    let content = r.unwrap();
    assert!(
        content.contains("rez"),
        "zsh completion must mention 'rez': {content}"
    );
}

#[test]
fn test_bash_complete_mentions_rez_command() {
    assert!(
        BASH_COMPLETE.contains("rez"),
        "BASH_COMPLETE must mention the rez command surface"
    );
}

#[test]
fn test_zsh_complete_has_compdef_directive() {
    assert!(ZSH_COMPLETE.contains("#compdef"), "ZSH_COMPLETE must contain #compdef directive");
}

#[test]
fn test_fish_complete_has_complete_c_directive() {
    assert!(FISH_COMPLETE.contains("complete -c"), "FISH_COMPLETE must use complete -c directives");
}

#[test]
fn test_get_completion_script_none_returns_ok() {
    let d = PyRezData::new();
    let r = d.get_completion_script(None);
    assert!(r.is_ok(), "get_completion_script(None) must not error");
    assert!(!r.unwrap().is_empty(), "fallback completion script must not be empty");
}

#[test]
fn test_get_default_config_contains_local_packages_path_key() {
    let d = PyRezData::new();
    let cfg = d.get_default_config();
    assert!(cfg.contains("local_packages_path"), "default config must contain local_packages_path");
}

#[test]
fn test_bash_complete_lists_build_subcommand() {
    assert!(BASH_COMPLETE.contains("build"), "bash completion must list build subcommand");
}

#[test]
fn test_list_data_resources_contains_completion_entry() {
    let resources = list_data_resources();
    let has_completion = resources.iter().any(|r| r.contains("bash") || r.contains("completion"));
    assert!(has_completion, "data resources must include a completion entry");
}
