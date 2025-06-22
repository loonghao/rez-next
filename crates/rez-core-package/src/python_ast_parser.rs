//! Advanced Python AST parser for package.py files using RustPython

use crate::Package;
use rez_core_common::RezCoreError;
use rez_core_version::Version;
use rustpython_ast::{Suite, Stmt, Expr, Constant};
use rustpython_parser::Parse;
use std::collections::HashMap;

/// Advanced Python AST parser for package.py files
pub struct PythonAstParser;

impl PythonAstParser {
    /// Parse a package.py file using Python AST
    pub fn parse_package_py(content: &str) -> Result<Package, RezCoreError> {
        // Parse the Python code into an AST
        let ast = Suite::parse(content, "package.py")
            .map_err(|e| RezCoreError::PackageParse(format!("Python syntax error: {}", e)))?;

        let mut package_data = PackageData::new();

        // Walk through the AST and extract package information
        for stmt in &ast {
            Self::process_statement(stmt, &mut package_data)?;
        }

        // Convert extracted data to Package
        Self::build_package(package_data)
    }

    /// Process a single AST statement
    fn process_statement(stmt: &Stmt, package_data: &mut PackageData) -> Result<(), RezCoreError> {
        match stmt {
            Stmt::Assign(assign) => {
                // Handle variable assignments like: name = "value"
                if let Some(target) = assign.targets.first() {
                    if let Expr::Name(name_expr) = target {
                        Self::process_assignment(&name_expr.id, &assign.value, package_data)?;
                    }
                }
            }
            Stmt::FunctionDef(func_def) => {
                // Handle function definitions like: def commands(): ...
                if func_def.name.as_str() == "commands" {
                    Self::process_commands_function(&func_def.body, package_data)?;
                }
            }
            _ => {
                // Ignore other statement types for now
            }
        }
        Ok(())
    }

    /// Process variable assignments
    fn process_assignment(var_name: &str, value: &Expr, package_data: &mut PackageData) -> Result<(), RezCoreError> {
        match var_name {
            "name" => {
                package_data.name = Some(Self::extract_string_value(value)?);
            }
            "version" => {
                package_data.version = Some(Self::extract_string_value(value)?);
            }
            "description" => {
                package_data.description = Some(Self::extract_string_value(value)?);
            }
            "build_command" => {
                package_data.build_command = Some(Self::extract_string_value(value)?);
            }
            "build_system" => {
                package_data.build_system = Some(Self::extract_string_value(value)?);
            }
            "uuid" => {
                package_data.uuid = Some(Self::extract_string_value(value)?);
            }
            "authors" => {
                package_data.authors = Self::extract_string_list(value)?;
            }
            "requires" => {
                package_data.requires = Self::extract_string_list(value)?;
            }
            "build_requires" => {
                package_data.build_requires = Self::extract_string_list(value)?;
            }
            "private_build_requires" => {
                package_data.private_build_requires = Self::extract_string_list(value)?;
            }
            "tools" => {
                package_data.tools = Self::extract_string_list(value)?;
            }
            "variants" => {
                package_data.variants = Self::extract_variants(value)?;
            }
            "tests" => {
                package_data.tests = Self::extract_tests(value)?;
            }
            "pre_commands" => {
                package_data.pre_commands = Some(Self::extract_string_value(value)?);
            }
            "post_commands" => {
                package_data.post_commands = Some(Self::extract_string_value(value)?);
            }
            "pre_test_commands" => {
                package_data.pre_test_commands = Some(Self::extract_string_value(value)?);
            }
            "pre_build_commands" => {
                package_data.pre_build_commands = Some(Self::extract_string_value(value)?);
            }
            "requires_rez_version" => {
                package_data.requires_rez_version = Some(Self::extract_string_value(value)?);
            }
            "help" => {
                package_data.help = Some(Self::extract_string_value(value)?);
            }
            "relocatable" => {
                package_data.relocatable = Self::extract_bool_value(value)?;
            }
            "cachable" => {
                package_data.cachable = Self::extract_bool_value(value)?;
            }
            "base" => {
                package_data.base = Some(Self::extract_string_value(value)?);
            }
            "hashed_variants" => {
                package_data.hashed_variants = Self::extract_bool_value(value)?;
            }
            "has_plugins" => {
                package_data.has_plugins = Self::extract_bool_value(value)?;
            }
            "plugin_for" => {
                package_data.plugin_for = Self::extract_string_list(value)?;
            }
            "format_version" => {
                package_data.format_version = Some(Self::extract_int_value(value)?);
            }
            "preprocess" => {
                package_data.preprocess = Some(Self::extract_string_value(value)?);
            }
            _ => {
                // Store unknown fields for later processing
                package_data.extra_fields.insert(var_name.to_string(), format!("{:?}", value));
            }
        }
        Ok(())
    }

    /// Extract string value from expression
    fn extract_string_value(expr: &Expr) -> Result<String, RezCoreError> {
        match expr {
            Expr::Constant(constant) => {
                match &constant.value {
                    Constant::Str(s) => Ok(s.clone()),
                    Constant::Int(i) => Ok(i.to_string()),
                    Constant::Float(f) => Ok(f.to_string()),
                    _ => Err(RezCoreError::PackageParse(format!("Expected string/number value, got: {:?}", constant.value)))
                }
            }
            _ => Err(RezCoreError::PackageParse(format!("Expected constant value, got: {:?}", expr)))
        }
    }

    /// Extract boolean value from expression
    fn extract_bool_value(expr: &Expr) -> Result<Option<bool>, RezCoreError> {
        match expr {
            Expr::Constant(constant) => {
                match &constant.value {
                    Constant::Bool(b) => Ok(Some(*b)),
                    Constant::None => Ok(None),
                    _ => Err(RezCoreError::PackageParse(format!("Expected boolean value, got: {:?}", constant.value)))
                }
            }
            _ => Err(RezCoreError::PackageParse(format!("Expected constant value, got: {:?}", expr)))
        }
    }

    /// Extract integer value from expression
    fn extract_int_value(expr: &Expr) -> Result<i32, RezCoreError> {
        match expr {
            Expr::Constant(constant) => {
                match &constant.value {
                    Constant::Int(i) => {
                        // Convert BigInt to i32 safely
                        i.to_string().parse::<i32>()
                            .map_err(|e| RezCoreError::PackageParse(format!("Integer too large for i32: {}", e)))
                    },
                    _ => Err(RezCoreError::PackageParse(format!("Expected integer value, got: {:?}", constant.value)))
                }
            }
            _ => Err(RezCoreError::PackageParse(format!("Expected constant value, got: {:?}", expr)))
        }
    }

    /// Extract list of strings from expression
    fn extract_string_list(expr: &Expr) -> Result<Vec<String>, RezCoreError> {
        match expr {
            Expr::List(list) => {
                let mut result = Vec::new();
                for elt in &list.elts {
                    result.push(Self::extract_string_value(elt)?);
                }
                Ok(result)
            }
            Expr::Tuple(tuple) => {
                let mut result = Vec::new();
                for elt in &tuple.elts {
                    result.push(Self::extract_string_value(elt)?);
                }
                Ok(result)
            }
            _ => Err(RezCoreError::PackageParse(format!("Expected list, got: {:?}", expr)))
        }
    }

    /// Extract variants (list of lists)
    fn extract_variants(expr: &Expr) -> Result<Vec<Vec<String>>, RezCoreError> {
        match expr {
            Expr::List(list) => {
                let mut result = Vec::new();
                for elt in &list.elts {
                    result.push(Self::extract_string_list(elt)?);
                }
                Ok(result)
            }
            _ => Err(RezCoreError::PackageParse(format!("Expected list of lists for variants, got: {:?}", expr)))
        }
    }

    /// Extract tests dictionary
    fn extract_tests(expr: &Expr) -> Result<HashMap<String, String>, RezCoreError> {
        match expr {
            Expr::Dict(dict) => {
                let mut result = HashMap::new();
                for (key, value) in dict.keys.iter().zip(dict.values.iter()) {
                    if let Some(key) = key {
                        let key_str = Self::extract_string_value(key)?;
                        let value_str = Self::extract_string_value(value)?;
                        result.insert(key_str, value_str);
                    }
                }
                Ok(result)
            }
            _ => Err(RezCoreError::PackageParse(format!("Expected dictionary for tests, got: {:?}", expr)))
        }
    }

    /// Process commands function
    fn process_commands_function(body: &[Stmt], package_data: &mut PackageData) -> Result<(), RezCoreError> {
        // Extract environment variable assignments and path modifications
        let mut commands = Vec::new();

        for stmt in body {
            if let Some(command) = Self::extract_command_from_statement(stmt)? {
                commands.push(command);
            }
        }

        if !commands.is_empty() {
            package_data.commands_function = Some(commands.join("\n"));
        }

        Ok(())
    }

    /// Extract command from a statement in commands function
    fn extract_command_from_statement(stmt: &Stmt) -> Result<Option<String>, RezCoreError> {
        match stmt {
            // Handle env.VAR = "value" or env.VAR.append("value")
            Stmt::Assign(assign) => {
                if let Some(target) = assign.targets.first() {
                    if let Expr::Attribute(attr) = target {
                        if let Expr::Name(name_expr) = &*attr.value {
                            if name_expr.id.as_str() == "env" {
                                let var_name = &attr.attr;
                                if let Some(value) = Self::extract_string_value(&assign.value).ok() {
                                    return Ok(Some(format!("export {}=\"{}\"", var_name, value)));
                                }
                            }
                        }
                    }
                }
            }
            // Handle env.PATH.append("value") or env.VAR.prepend("value")
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    if let Expr::Attribute(attr) = &*call.func {
                        if let Expr::Attribute(env_attr) = &*attr.value {
                            if let Expr::Name(name_expr) = &*env_attr.value {
                                if name_expr.id.as_str() == "env" {
                                    let var_name = &env_attr.attr;
                                    let method = &attr.attr;

                                    if let Some(arg) = call.args.first() {
                                        if let Ok(value) = Self::extract_string_value(arg) {
                                            match method.as_str() {
                                                "append" => return Ok(Some(format!("export {}=\"${{{}}}:{}\"", var_name, var_name, value))),
                                                "prepend" => return Ok(Some(format!("export {}=\"{}:${{{}}}\"", var_name, value, var_name))),
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }

    /// Build Package from extracted data
    fn build_package(data: PackageData) -> Result<Package, RezCoreError> {
        let name = data.name.ok_or_else(|| RezCoreError::PackageParse("Missing 'name' field".to_string()))?;
        
        let mut package = Package::new(name);

        // Set version
        if let Some(version_str) = data.version {
            package.version = Some(Version::parse(&version_str)
                .map_err(|e| RezCoreError::PackageParse(format!("Invalid version: {}", e)))?);
        }

        // Set other fields
        package.description = data.description;
        package.build_command = data.build_command;
        package.build_system = data.build_system;
        package.pre_commands = data.pre_commands;
        package.post_commands = data.post_commands;
        package.pre_test_commands = data.pre_test_commands;
        package.pre_build_commands = data.pre_build_commands;
        package.tests = data.tests;
        package.requires_rez_version = data.requires_rez_version;
        package.uuid = data.uuid;
        package.authors = data.authors;
        package.requires = data.requires;
        package.build_requires = data.build_requires;
        package.private_build_requires = data.private_build_requires;
        package.tools = data.tools;
        package.variants = data.variants;
        package.help = data.help;
        package.relocatable = data.relocatable;
        package.cachable = data.cachable;
        package.commands = data.commands_function;

        // Set new fields for complete rez compatibility
        package.base = data.base;
        package.hashed_variants = data.hashed_variants;
        package.has_plugins = data.has_plugins;
        package.plugin_for = data.plugin_for;
        package.format_version = data.format_version;
        package.preprocess = data.preprocess;

        // Validate the package
        package.validate()?;

        Ok(package)
    }
}

/// Intermediate data structure for collecting package information
#[derive(Debug, Default)]
struct PackageData {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    build_command: Option<String>,
    build_system: Option<String>,
    pre_commands: Option<String>,
    post_commands: Option<String>,
    pre_test_commands: Option<String>,
    pre_build_commands: Option<String>,
    tests: HashMap<String, String>,
    requires_rez_version: Option<String>,
    uuid: Option<String>,
    authors: Vec<String>,
    requires: Vec<String>,
    build_requires: Vec<String>,
    private_build_requires: Vec<String>,
    tools: Vec<String>,
    variants: Vec<Vec<String>>,
    help: Option<String>,
    relocatable: Option<bool>,
    cachable: Option<bool>,
    commands_function: Option<String>,
    extra_fields: HashMap<String, String>,
    // New fields for complete rez compatibility
    base: Option<String>,
    hashed_variants: Option<bool>,
    has_plugins: Option<bool>,
    plugin_for: Vec<String>,
    format_version: Option<i32>,
    preprocess: Option<String>,
}

impl PackageData {
    fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_with_new_fields() {
        let package_py_content = r#"
name = "test_package"
version = "1.0.0"
description = "Test package with new fields"
base = "base_package"
hashed_variants = True
has_plugins = True
plugin_for = ["maya", "nuke"]
format_version = 2
preprocess = "some_preprocess_function"
"#;

        let result = PythonAstParser::parse_package_py(package_py_content);
        assert!(result.is_ok(), "Failed to parse package.py: {:?}", result.err());

        let package = result.unwrap();
        assert_eq!(package.name, "test_package");
        assert_eq!(package.base, Some("base_package".to_string()));
        assert_eq!(package.hashed_variants, Some(true));
        assert_eq!(package.has_plugins, Some(true));
        assert_eq!(package.plugin_for, vec!["maya", "nuke"]);
        assert_eq!(package.format_version, Some(2));
        assert_eq!(package.preprocess, Some("some_preprocess_function".to_string()));
    }

    #[test]
    fn test_parse_package_with_false_boolean_fields() {
        let package_py_content = r#"
name = "test_package"
version = "1.0.0"
hashed_variants = False
has_plugins = False
"#;

        let result = PythonAstParser::parse_package_py(package_py_content);
        assert!(result.is_ok(), "Failed to parse package.py: {:?}", result.err());

        let package = result.unwrap();
        assert_eq!(package.hashed_variants, Some(false));
        assert_eq!(package.has_plugins, Some(false));
    }
}
