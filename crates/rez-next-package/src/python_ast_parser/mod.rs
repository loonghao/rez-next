//! Advanced Python AST parser for package.py files using RustPython.
//!
//! This module is split into focused submodules:
//! - `types`: Core data types (`PythonValue`, `ParsingContext`, `PackageData`)
//! - `eval`: Expression evaluation (`evaluate_*`)
//! - `extract`: Typed value extraction (`extract_*`)
//! - `commands`: Rex DSL command parsing (`process_commands_function` etc.)

mod commands;
mod eval;
mod extract;
mod types;

use crate::Package;
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use rustpython_ast::{Expr, Stmt, Suite};
use rustpython_parser::Parse;
use types::{PackageData, ParsingContext};

/// Advanced Python AST parser for package.py files
#[derive(Default)]
pub struct PythonAstParser {
    /// Context for tracking variables and imports during parsing
    pub(crate) context: ParsingContext,
}

impl PythonAstParser {
    /// Create a new parser instance
    pub fn new() -> Self {
        Self {
            context: ParsingContext::default(),
        }
    }

    /// Parse a package.py file using Python AST
    pub fn parse_package_py(content: &str) -> Result<Package, RezCoreError> {
        let mut parser = Self::new();
        parser.parse_package_py_with_context(content)
    }

    /// Parse a package.py file with context tracking
    fn parse_package_py_with_context(&mut self, content: &str) -> Result<Package, RezCoreError> {
        let ast = Suite::parse(content, "package.py")
            .map_err(|e| RezCoreError::PackageParse(format!("Python syntax error: {}", e)))?;

        let mut package_data = PackageData::new();

        for stmt in &ast {
            self.process_statement(stmt, &mut package_data)?;
        }

        Self::build_package(package_data)
    }

    /// Process a single AST statement
    fn process_statement(
        &mut self,
        stmt: &Stmt,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        match stmt {
            Stmt::Assign(assign) => {
                if let Some(Expr::Name(name_expr)) = assign.targets.first() {
                    self.process_assignment(&name_expr.id, &assign.value, package_data)?;
                }
            }
            Stmt::FunctionDef(func_def) => {
                self.process_function_definition(func_def, package_data)?;
            }
            Stmt::Import(import) => {
                self.process_import_statement(import)?;
            }
            Stmt::ImportFrom(import_from) => {
                self.process_import_from_statement(import_from)?;
            }
            Stmt::If(if_stmt) => {
                self.process_if_statement(if_stmt, package_data)?;
            }
            Stmt::For(for_stmt) => {
                self.process_for_statement(for_stmt, package_data)?;
            }
            Stmt::While(while_stmt) => {
                self.process_while_statement(while_stmt, package_data)?;
            }
            Stmt::Try(try_stmt) => {
                self.process_try_statement(try_stmt, package_data)?;
            }
            Stmt::With(with_stmt) => {
                self.process_with_statement(with_stmt, package_data)?;
            }
            Stmt::Expr(expr_stmt) => {
                self.process_expression_statement(&expr_stmt.value, package_data)?;
            }
            Stmt::Pass(_) => {}
            _ => {
                return Err(RezCoreError::PackageParse(format!(
                    "Unsupported package.py statement: {stmt:?}"
                )));
            }
        }
        Ok(())
    }

    /// Process import statements (`import os`)
    fn process_import_statement(
        &mut self,
        import: &rustpython_ast::StmtImport,
    ) -> Result<(), RezCoreError> {
        for alias in &import.names {
            let module_name = alias.name.as_str();
            let alias_name = alias
                .asname
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or(module_name);
            self.context
                .imports
                .insert(alias_name.to_string(), module_name.to_string());
        }
        Ok(())
    }

    /// Process from-import statements (`from os import path`)
    fn process_import_from_statement(
        &mut self,
        import_from: &rustpython_ast::StmtImportFrom,
    ) -> Result<(), RezCoreError> {
        if let Some(module) = &import_from.module {
            for alias in &import_from.names {
                let name = alias.name.as_str();
                let alias_name = alias.asname.as_ref().map(|s| s.as_str()).unwrap_or(name);
                let full_name = format!("{}.{}", module, name);
                self.context
                    .imports
                    .insert(alias_name.to_string(), full_name);
            }
        }
        Ok(())
    }

    /// Process function definitions (`def commands(): ...`)
    fn process_function_definition(
        &mut self,
        func_def: &rustpython_ast::StmtFunctionDef,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        match func_def.name.as_str() {
            "commands" => {
                self.process_commands_function(&func_def.body, package_data)?;
            }
            "pre_commands" => {
                self.process_pre_commands_function(&func_def.body, package_data)?;
            }
            "post_commands" => {
                self.process_post_commands_function(&func_def.body, package_data)?;
            }
            _ => {
                return Err(RezCoreError::PackageParse(format!(
                    "Unsupported package.py function: {}",
                    func_def.name
                )));
            }
        }
        Ok(())
    }

    /// Process conditional statements
    fn process_if_statement(
        &mut self,
        if_stmt: &rustpython_ast::StmtIf,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        match self.evaluate_expression(&if_stmt.test)? {
            types::PythonValue::Boolean(true) => {
                for stmt in &if_stmt.body {
                    self.process_statement(stmt, package_data)?;
                }
            }
            types::PythonValue::Boolean(false) => {
                for stmt in &if_stmt.orelse {
                    self.process_statement(stmt, package_data)?;
                }
            }
            _ => {
                return Err(RezCoreError::PackageParse(
                    "Package condition is not statically boolean".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Process variable assignments and map them to `PackageData` fields
    fn process_assignment(
        &mut self,
        var_name: &str,
        value: &Expr,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        let python_value = self.evaluate_expression(value)?;
        self.context
            .variables
            .insert(var_name.to_string(), python_value);

        match var_name {
            "name" => {
                package_data.name = Some(self.extract_string_value(value)?);
            }
            "version" => {
                package_data.version = Some(self.extract_string_value(value)?);
            }
            "description" => {
                package_data.description = Some(self.extract_string_value(value)?);
            }
            "build_command" => {
                package_data.build_command = Some(self.extract_string_value(value)?);
            }
            "build_system" => {
                package_data.build_system = Some(self.extract_string_value(value)?);
            }
            "uuid" => {
                package_data.uuid = Some(self.extract_string_value(value)?);
            }
            "authors" => {
                package_data.authors = self.extract_string_list(value)?;
            }
            "requires" => {
                package_data.requires = self.extract_string_list(value)?;
            }
            "build_requires" => {
                package_data.build_requires = self.extract_string_list(value)?;
            }
            "private_build_requires" => {
                package_data.private_build_requires = self.extract_string_list(value)?;
            }
            "tools" => {
                package_data.tools = self.extract_string_list(value)?;
            }
            "variants" => {
                package_data.variants = self.extract_variants(value)?;
            }
            "tests" => {
                package_data.tests = self.extract_tests(value)?;
            }
            "commands" => {
                if let Ok(s) = self.extract_string_value(value) {
                    package_data.commands_function = Some(s);
                }
            }
            "pre_commands" => {
                package_data.pre_commands = Some(self.extract_string_value(value)?);
            }
            "post_commands" => {
                package_data.post_commands = Some(self.extract_string_value(value)?);
            }
            "pre_test_commands" => {
                package_data.pre_test_commands = Some(self.extract_string_value(value)?);
            }
            "pre_build_commands" => {
                package_data.pre_build_commands = Some(self.extract_string_value(value)?);
            }
            "requires_rez_version" => {
                package_data.requires_rez_version = Some(self.extract_string_value(value)?);
            }
            "help" => {
                package_data.help = Some(self.extract_string_value(value)?);
            }
            "relocatable" => {
                package_data.relocatable = self.extract_bool_value(value)?;
            }
            "cachable" => {
                package_data.cachable = self.extract_bool_value(value)?;
            }
            "base" => {
                package_data.base = Some(self.extract_string_value(value)?);
            }
            "hashed_variants" => {
                package_data.hashed_variants = self.extract_bool_value(value)?;
            }
            "has_plugins" => {
                package_data.has_plugins = self.extract_bool_value(value)?;
            }
            "plugin_for" => {
                package_data.plugin_for = self.extract_string_list(value)?;
            }
            "format_version" => {
                package_data.format_version = Some(self.extract_int_value(value)?);
            }
            "preprocess" => {
                package_data.preprocess = Some(self.extract_string_value(value)?);
            }
            _ => {
                package_data
                    .extra_fields
                    .insert(var_name.to_string(), format!("{:?}", value));
            }
        }
        Ok(())
    }

    /// Reject loops because traversing their body once corrupts package metadata.
    fn process_for_statement(
        &mut self,
        for_stmt: &rustpython_ast::StmtFor,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        let _ = package_data;
        Err(RezCoreError::PackageParse(format!(
            "Unsupported package.py for loop: {for_stmt:?}"
        )))
    }

    /// Reject loops because traversing their body once corrupts package metadata.
    fn process_while_statement(
        &mut self,
        while_stmt: &rustpython_ast::StmtWhile,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        let _ = package_data;
        Err(RezCoreError::PackageParse(format!(
            "Unsupported package.py while loop: {while_stmt:?}"
        )))
    }

    /// Reject try/except because evaluating every branch corrupts package metadata.
    fn process_try_statement(
        &mut self,
        try_stmt: &rustpython_ast::StmtTry,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        let _ = package_data;
        Err(RezCoreError::PackageParse(format!(
            "Unsupported package.py try statement: {try_stmt:?}"
        )))
    }

    /// Process with statements
    fn process_with_statement(
        &mut self,
        with_stmt: &rustpython_ast::StmtWith,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        let _ = package_data;
        Err(RezCoreError::PackageParse(format!(
            "Unsupported package.py with statement: {with_stmt:?}"
        )))
    }

    /// Process standalone expression statements (evaluate for side-effects only)
    fn process_expression_statement(
        &mut self,
        expr: &Expr,
        _package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        self.evaluate_expression(expr)?;
        Ok(())
    }

    /// Build a `Package` from the collected `PackageData`
    fn build_package(data: PackageData) -> Result<Package, RezCoreError> {
        let name = data
            .name
            .ok_or_else(|| RezCoreError::PackageParse("Missing 'name' field".to_string()))?;

        let mut package = Package::new(name);

        if let Some(version_str) = data.version {
            package.version = Some(
                Version::parse(&version_str)
                    .map_err(|e| RezCoreError::PackageParse(format!("Invalid version: {}", e)))?,
            );
        }

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
        package.commands = data.commands_function.clone();
        package.commands_function = data.commands_function;
        package.base = data.base;
        package.hashed_variants = data.hashed_variants;
        package.has_plugins = data.has_plugins;
        package.plugin_for = data.plugin_for;
        package.format_version = data.format_version;
        package.preprocess = data.preprocess;

        package.validate()?;

        Ok(package)
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
        assert!(
            result.is_ok(),
            "Failed to parse package.py: {:?}",
            result.err()
        );

        let package = result.unwrap();
        assert_eq!(package.name, "test_package");
        assert_eq!(package.base, Some("base_package".to_string()));
        assert_eq!(package.hashed_variants, Some(true));
        assert_eq!(package.has_plugins, Some(true));
        assert_eq!(package.plugin_for, vec!["maya", "nuke"]);
        assert_eq!(package.format_version, Some(2));
        assert_eq!(
            package.preprocess,
            Some("some_preprocess_function".to_string())
        );
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
        assert!(
            result.is_ok(),
            "Failed to parse package.py: {:?}",
            result.err()
        );

        let package = result.unwrap();
        assert_eq!(package.hashed_variants, Some(false));
        assert_eq!(package.has_plugins, Some(false));
    }

    #[test]
    fn test_parse_package_with_conditional_logic() {
        let package_py_content = r#"
name = "test_package"
version = "1.0.0"

import os
if os.name == "nt":
    requires = ["windows-lib"]
else:
    requires = ["unix-lib"]

def commands():
    env.PATH.append("/usr/local/bin")
    env.PYTHONPATH.prepend("/opt/python")
"#;

        let result = PythonAstParser::parse_package_py(package_py_content);
        assert!(
            result.is_ok(),
            "Failed to parse package.py: {:?}",
            result.err()
        );

        let package = result.unwrap();
        assert_eq!(package.name, "test_package");
        assert!(!package.requires.is_empty());
    }

    #[test]
    fn test_parse_package_with_expressions() {
        let package_py_content = r#"
name = "test_package"
version = "1.0.0"

base_version = "2.0"
version = base_version + ".1"

authors = ["author1"] + ["author2"]
"#;

        let result = PythonAstParser::parse_package_py(package_py_content);
        assert!(
            result.is_ok(),
            "Failed to parse package.py: {:?}",
            result.err()
        );

        let package = result.unwrap();
        assert_eq!(package.name, "test_package");
    }

    #[test]
    fn test_commands_select_os_getenv_default_branch() {
        let package = PythonAstParser::parse_package_py(
            r#"
name = "test_package"
version = "1.0.0"
import os
def commands():
    if os.getenv("REZ_NEXT_TEST_UNSET", "false").lower() == "true":
        env.MODE = "enabled"
    else:
        env.MODE = "disabled"
"#,
        )
        .unwrap();

        assert_eq!(
            package.commands.as_deref(),
            Some("env.setenv('MODE', 'disabled')")
        );
    }

    #[test]
    fn test_commands_evaluate_platform_alias_with_local_variables() {
        let package = PythonAstParser::parse_package_py(
            r#"
name = "test_package"
version = "15.2.1"
def commands():
    extension = ".exe" if system.platform == "windows" else ""
    executable = "Tool{{this.version.major}}.{{this.version.minor}}{0}".format(extension)
    if system.platform == "windows":
        alias("tool", "{0} $*".format(executable))
    else:
        alias("tool", "{0} $@".format(executable))
"#,
        )
        .unwrap();

        let suffix = if cfg!(windows) { ".exe $*" } else { " $@" };
        assert_eq!(
            package.commands.as_deref(),
            Some(format!(
                "alias('tool', 'Tool{{this.version.major}}.{{this.version.minor}}{suffix}')"
            ))
            .as_deref()
        );
    }

    #[test]
    fn test_dynamic_metadata_is_rejected() {
        let error = PythonAstParser::parse_package_py(
            r#"
name = "test_package"
version = load_version()
"#,
        )
        .expect_err("dynamic metadata must not be converted into debug text");

        assert!(
            error.to_string().contains("Unsupported function call"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_unknown_command_is_rejected() {
        let error = PythonAstParser::parse_package_py(
            r#"
name = "test_package"
version = "1.0.0"
def commands():
    env.PATH.extend("/unsupported")
"#,
        )
        .expect_err("unsupported commands must not disappear from the environment");

        assert!(
            error.to_string().contains("Unsupported command statement"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_dynamic_metadata_function_is_rejected() {
        let error = PythonAstParser::parse_package_py(
            r#"
name = "test_package"
@early()
def version():
    return "1.0.0"
"#,
        )
        .expect_err("dynamic metadata functions require Python execution");

        assert!(
            error
                .to_string()
                .contains("Unsupported package.py function"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_dynamic_control_flow_is_rejected() {
        let error = PythonAstParser::parse_package_py(
            r#"
name = "test_package"
version = "1.0.0"
for requirement in discover_requirements():
    requires = [requirement]
"#,
        )
        .expect_err("loops must not be approximated by executing their body once");

        assert!(
            error
                .to_string()
                .contains("Unsupported package.py for loop"),
            "unexpected error: {error}"
        );
    }
}
