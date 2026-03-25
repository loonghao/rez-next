//! Advanced Python AST parser for package.py files using RustPython

use crate::Package;
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use rustpython_ast::{BoolOp, CmpOp, Constant, Expr, Operator, Stmt, Suite, UnaryOp};
use rustpython_parser::Parse;
use std::collections::HashMap;

/// Advanced Python AST parser for package.py files
pub struct PythonAstParser {
    /// Context for tracking variables and imports during parsing
    context: ParsingContext,
}

/// Context for tracking variables and imports during parsing
#[derive(Debug, Default)]
struct ParsingContext {
    /// Variables defined in the current scope
    variables: HashMap<String, PythonValue>,
    /// Imported modules and their aliases
    imports: HashMap<String, String>,
    /// Current function scope (for nested function handling)
    function_scope: Vec<String>,
}

/// Represents a Python value that can be evaluated
#[derive(Debug, Clone)]
enum PythonValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    List(Vec<PythonValue>),
    Dict(HashMap<String, PythonValue>),
    None,
    /// For complex expressions that need runtime evaluation
    Expression(String),
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
        // Parse the Python code into an AST
        let ast = Suite::parse(content, "package.py")
            .map_err(|e| RezCoreError::PackageParse(format!("Python syntax error: {}", e)))?;

        let mut package_data = PackageData::new();

        // Walk through the AST and extract package information
        for stmt in &ast {
            self.process_statement(stmt, &mut package_data)?;
        }

        // Convert extracted data to Package
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
                // Handle variable assignments like: name = "value"
                if let Some(target) = assign.targets.first() {
                    if let Expr::Name(name_expr) = target {
                        self.process_assignment(&name_expr.id, &assign.value, package_data)?;
                    }
                }
            }
            Stmt::FunctionDef(func_def) => {
                // Handle function definitions like: def commands(): ...
                self.process_function_definition(func_def, package_data)?;
            }
            Stmt::Import(import) => {
                // Handle import statements like: import os
                self.process_import_statement(import)?;
            }
            Stmt::ImportFrom(import_from) => {
                // Handle from imports like: from os import path
                self.process_import_from_statement(import_from)?;
            }
            Stmt::If(if_stmt) => {
                // Handle conditional statements
                self.process_if_statement(if_stmt, package_data)?;
            }
            Stmt::For(for_stmt) => {
                // Handle for loops
                self.process_for_statement(for_stmt, package_data)?;
            }
            Stmt::While(while_stmt) => {
                // Handle while loops
                self.process_while_statement(while_stmt, package_data)?;
            }
            Stmt::Try(try_stmt) => {
                // Handle try/except blocks
                self.process_try_statement(try_stmt, package_data)?;
            }
            Stmt::With(with_stmt) => {
                // Handle with statements
                self.process_with_statement(with_stmt, package_data)?;
            }
            Stmt::Expr(expr_stmt) => {
                // Handle standalone expressions
                self.process_expression_statement(&expr_stmt.value, package_data)?;
            }
            _ => {
                // Log unhandled statement types for debugging
                eprintln!("Unhandled statement type: {:?}", stmt);
            }
        }
        Ok(())
    }

    /// Process import statements
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

    /// Process from import statements
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

    /// Process function definitions
    fn process_function_definition(
        &mut self,
        func_def: &rustpython_ast::StmtFunctionDef,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        self.context.function_scope.push(func_def.name.to_string());

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
                // Store other function definitions for potential late binding
                package_data.functions.insert(
                    func_def.name.to_string(),
                    self.function_to_string(func_def)?,
                );
            }
        }

        self.context.function_scope.pop();
        Ok(())
    }

    /// Process conditional statements
    fn process_if_statement(
        &mut self,
        if_stmt: &rustpython_ast::StmtIf,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        // Evaluate the condition if possible
        if let Ok(condition_result) = self.evaluate_expression(&if_stmt.test) {
            match condition_result {
                PythonValue::Boolean(true) => {
                    // Execute the if body
                    for stmt in &if_stmt.body {
                        self.process_statement(stmt, package_data)?;
                    }
                }
                PythonValue::Boolean(false) => {
                    // Execute the else body if present
                    for stmt in &if_stmt.orelse {
                        self.process_statement(stmt, package_data)?;
                    }
                }
                _ => {
                    // If we can't evaluate the condition, process both branches
                    // This is a conservative approach for complex conditions
                    for stmt in &if_stmt.body {
                        self.process_statement(stmt, package_data)?;
                    }
                    for stmt in &if_stmt.orelse {
                        self.process_statement(stmt, package_data)?;
                    }
                }
            }
        } else {
            // If condition evaluation fails, process both branches
            for stmt in &if_stmt.body {
                self.process_statement(stmt, package_data)?;
            }
            for stmt in &if_stmt.orelse {
                self.process_statement(stmt, package_data)?;
            }
        }
        Ok(())
    }

    /// Process variable assignments
    fn process_assignment(
        &mut self,
        var_name: &str,
        value: &Expr,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        // First, try to evaluate the expression and store in context
        if let Ok(python_value) = self.evaluate_expression(value) {
            self.context
                .variables
                .insert(var_name.to_string(), python_value.clone());
        }

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
                // Store unknown fields for later processing
                package_data
                    .extra_fields
                    .insert(var_name.to_string(), format!("{:?}", value));
            }
        }
        Ok(())
    }

    /// Process for loops
    fn process_for_statement(
        &mut self,
        for_stmt: &rustpython_ast::StmtFor,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        // For now, we'll process the body without iteration
        // In a full implementation, we'd need to evaluate the iterator
        for stmt in &for_stmt.body {
            self.process_statement(stmt, package_data)?;
        }
        Ok(())
    }

    /// Process while loops
    fn process_while_statement(
        &mut self,
        while_stmt: &rustpython_ast::StmtWhile,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        // For now, we'll process the body once
        // In a full implementation, we'd need to evaluate the condition
        for stmt in &while_stmt.body {
            self.process_statement(stmt, package_data)?;
        }
        Ok(())
    }

    /// Process try/except blocks
    fn process_try_statement(
        &mut self,
        try_stmt: &rustpython_ast::StmtTry,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        // Process the try body
        for stmt in &try_stmt.body {
            self.process_statement(stmt, package_data)?;
        }
        // Process exception handlers
        for handler in &try_stmt.handlers {
            match handler {
                rustpython_ast::ExceptHandler::ExceptHandler(eh) => {
                    for stmt in &eh.body {
                        self.process_statement(stmt, package_data)?;
                    }
                }
            }
        }
        // Process else clause
        for stmt in &try_stmt.orelse {
            self.process_statement(stmt, package_data)?;
        }
        // Process finally clause
        for stmt in &try_stmt.finalbody {
            self.process_statement(stmt, package_data)?;
        }
        Ok(())
    }

    /// Process with statements
    fn process_with_statement(
        &mut self,
        with_stmt: &rustpython_ast::StmtWith,
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        // Process the with body
        for stmt in &with_stmt.body {
            self.process_statement(stmt, package_data)?;
        }
        Ok(())
    }

    /// Process expression statements
    fn process_expression_statement(
        &mut self,
        expr: &Expr,
        _package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        // Evaluate the expression for side effects
        let _ = self.evaluate_expression(expr);
        Ok(())
    }

    /// Evaluate a Python expression to a value
    fn evaluate_expression(&self, expr: &Expr) -> Result<PythonValue, RezCoreError> {
        match expr {
            Expr::Constant(constant) => self.evaluate_constant(&constant.value),
            Expr::Name(name) => {
                // Look up variable in context
                if let Some(value) = self.context.variables.get(name.id.as_str()) {
                    Ok(value.clone())
                } else {
                    Err(RezCoreError::PackageParse(format!(
                        "Undefined variable: {}",
                        name.id
                    )))
                }
            }
            Expr::BinOp(binop) => self.evaluate_binary_operation(binop),
            Expr::UnaryOp(unaryop) => self.evaluate_unary_operation(unaryop),
            Expr::Compare(compare) => self.evaluate_comparison(compare),
            Expr::BoolOp(boolop) => self.evaluate_boolean_operation(boolop),
            Expr::List(list) => self.evaluate_list(list),
            Expr::Tuple(tuple) => self.evaluate_tuple(tuple),
            Expr::Dict(dict) => self.evaluate_dict(dict),
            Expr::Call(call) => self.evaluate_function_call(call),
            Expr::Attribute(attr) => self.evaluate_attribute(attr),
            Expr::Subscript(subscript) => self.evaluate_subscript(subscript),
            _ => {
                // For complex expressions, return as string representation
                Ok(PythonValue::Expression(format!("{:?}", expr)))
            }
        }
    }

    /// Extract string value from expression
    fn extract_string_value(&self, expr: &Expr) -> Result<String, RezCoreError> {
        // First try to evaluate the expression using context
        match self.evaluate_expression(expr) {
            Ok(PythonValue::String(s)) => Ok(s),
            Ok(PythonValue::Integer(i)) => Ok(i.to_string()),
            Ok(PythonValue::Float(f)) => Ok(f.to_string()),
            Ok(PythonValue::Boolean(b)) => Ok(b.to_string()),
            Ok(PythonValue::Expression(s)) => Ok(s),
            _ => {
                // Fallback to direct constant extraction
                match expr {
                    Expr::Constant(constant) => match &constant.value {
                        Constant::Str(s) => Ok(s.clone()),
                        Constant::Int(i) => Ok(i.to_string()),
                        Constant::Float(f) => Ok(f.to_string()),
                        _ => Err(RezCoreError::PackageParse(format!(
                            "Expected string/number value, got: {:?}",
                            constant.value
                        ))),
                    },
                    _ => Err(RezCoreError::PackageParse(format!(
                        "Expected constant value, got: {:?}",
                        expr
                    ))),
                }
            }
        }
    }

    /// Extract boolean value from expression
    fn extract_bool_value(&self, expr: &Expr) -> Result<Option<bool>, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::Boolean(b)) => Ok(Some(b)),
            Ok(PythonValue::None) => Ok(None),
            _ => {
                // Fallback to direct constant extraction
                match expr {
                    Expr::Constant(constant) => match &constant.value {
                        Constant::Bool(b) => Ok(Some(*b)),
                        Constant::None => Ok(None),
                        _ => Err(RezCoreError::PackageParse(format!(
                            "Expected boolean value, got: {:?}",
                            constant.value
                        ))),
                    },
                    _ => Err(RezCoreError::PackageParse(format!(
                        "Expected constant value, got: {:?}",
                        expr
                    ))),
                }
            }
        }
    }

    /// Extract integer value from expression
    fn extract_int_value(&self, expr: &Expr) -> Result<i32, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::Integer(i)) => Ok(i as i32),
            _ => {
                // Fallback to direct constant extraction
                match expr {
                    Expr::Constant(constant) => {
                        match &constant.value {
                            Constant::Int(i) => {
                                // Convert BigInt to i32 safely
                                i.to_string().parse::<i32>().map_err(|e| {
                                    RezCoreError::PackageParse(format!(
                                        "Integer too large for i32: {}",
                                        e
                                    ))
                                })
                            }
                            _ => Err(RezCoreError::PackageParse(format!(
                                "Expected integer value, got: {:?}",
                                constant.value
                            ))),
                        }
                    }
                    _ => Err(RezCoreError::PackageParse(format!(
                        "Expected constant value, got: {:?}",
                        expr
                    ))),
                }
            }
        }
    }

    /// Extract list of strings from expression
    fn extract_string_list(&self, expr: &Expr) -> Result<Vec<String>, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::List(list)) => {
                let mut result = Vec::new();
                for item in list {
                    match item {
                        PythonValue::String(s) => result.push(s),
                        PythonValue::Integer(i) => result.push(i.to_string()),
                        PythonValue::Float(f) => result.push(f.to_string()),
                        PythonValue::Boolean(b) => result.push(b.to_string()),
                        _ => {
                            return Err(RezCoreError::PackageParse(
                                "List contains non-string values".to_string(),
                            ))
                        }
                    }
                }
                Ok(result)
            }
            _ => {
                // Fallback to direct extraction
                match expr {
                    Expr::List(list) => {
                        let mut result = Vec::new();
                        for elt in &list.elts {
                            result.push(self.extract_string_value(elt)?);
                        }
                        Ok(result)
                    }
                    Expr::Tuple(tuple) => {
                        let mut result = Vec::new();
                        for elt in &tuple.elts {
                            result.push(self.extract_string_value(elt)?);
                        }
                        Ok(result)
                    }
                    _ => Err(RezCoreError::PackageParse(format!(
                        "Expected list, got: {:?}",
                        expr
                    ))),
                }
            }
        }
    }

    /// Extract variants (list of lists)
    fn extract_variants(&self, expr: &Expr) -> Result<Vec<Vec<String>>, RezCoreError> {
        match expr {
            Expr::List(list) => {
                let mut result = Vec::new();
                for elt in &list.elts {
                    result.push(self.extract_string_list(elt)?);
                }
                Ok(result)
            }
            _ => Err(RezCoreError::PackageParse(format!(
                "Expected list of lists for variants, got: {:?}",
                expr
            ))),
        }
    }

    /// Extract tests dictionary
    fn extract_tests(&self, expr: &Expr) -> Result<HashMap<String, String>, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::Dict(dict)) => {
                let mut result = HashMap::new();
                for (key, value) in dict {
                    let value_str = match value {
                        PythonValue::String(s) => s,
                        PythonValue::Integer(i) => i.to_string(),
                        PythonValue::Float(f) => f.to_string(),
                        PythonValue::Boolean(b) => b.to_string(),
                        _ => format!("{:?}", value),
                    };
                    result.insert(key, value_str);
                }
                Ok(result)
            }
            _ => {
                // Fallback to direct extraction
                match expr {
                    Expr::Dict(dict) => {
                        let mut result = HashMap::new();
                        for (key, value) in dict.keys.iter().zip(dict.values.iter()) {
                            if let Some(key) = key {
                                let key_str = self.extract_string_value(key)?;
                                let value_str = self.extract_string_value(value)?;
                                result.insert(key_str, value_str);
                            }
                        }
                        Ok(result)
                    }
                    _ => Err(RezCoreError::PackageParse(format!(
                        "Expected dictionary for tests, got: {:?}",
                        expr
                    ))),
                }
            }
        }
    }

    /// Process pre_commands function
    fn process_pre_commands_function(
        &mut self,
        body: &[Stmt],
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        let mut commands = Vec::new();
        for stmt in body {
            if let Some(command) = self.extract_command_from_statement(stmt)? {
                commands.push(command);
            }
        }
        if !commands.is_empty() {
            package_data.pre_commands = Some(commands.join("\n"));
        }
        Ok(())
    }

    /// Process post_commands function
    fn process_post_commands_function(
        &mut self,
        body: &[Stmt],
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        let mut commands = Vec::new();
        for stmt in body {
            if let Some(command) = self.extract_command_from_statement(stmt)? {
                commands.push(command);
            }
        }
        if !commands.is_empty() {
            package_data.post_commands = Some(commands.join("\n"));
        }
        Ok(())
    }

    /// Convert function definition to string representation
    fn function_to_string(
        &self,
        func_def: &rustpython_ast::StmtFunctionDef,
    ) -> Result<String, RezCoreError> {
        // This is a simplified implementation
        // In a full implementation, we'd reconstruct the Python code
        Ok(format!("def {}(): ...", func_def.name))
    }

    /// Process commands function
    fn process_commands_function(
        &mut self,
        body: &[Stmt],
        package_data: &mut PackageData,
    ) -> Result<(), RezCoreError> {
        // Extract environment variable assignments and path modifications
        let mut commands = Vec::new();

        for stmt in body {
            if let Some(command) = self.extract_command_from_statement(stmt)? {
                commands.push(command);
            }
        }

        if !commands.is_empty() {
            package_data.commands_function = Some(commands.join("\n"));
        }

        Ok(())
    }

    /// Extract command from a statement in commands function
    fn extract_command_from_statement(&self, stmt: &Stmt) -> Result<Option<String>, RezCoreError> {
        match stmt {
            // Handle env.VAR = "value" or env.VAR.append("value")
            Stmt::Assign(assign) => {
                if let Some(target) = assign.targets.first() {
                    if let Expr::Attribute(attr) = target {
                        if let Expr::Name(name_expr) = &*attr.value {
                            if name_expr.id.as_str() == "env" {
                                let var_name = &attr.attr;
                                if let Some(value) = self.extract_string_value(&assign.value).ok() {
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
                                        if let Ok(value) = self.extract_string_value(arg) {
                                            match method.as_str() {
                                                "append" => {
                                                    return Ok(Some(format!(
                                                        "export {}=\"${{{}}}:{}\"",
                                                        var_name, var_name, value
                                                    )))
                                                }
                                                "prepend" => {
                                                    return Ok(Some(format!(
                                                        "export {}=\"{}:${{{}}}\"",
                                                        var_name, value, var_name
                                                    )))
                                                }
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
        let name = data
            .name
            .ok_or_else(|| RezCoreError::PackageParse("Missing 'name' field".to_string()))?;

        let mut package = Package::new(name);

        // Set version
        if let Some(version_str) = data.version {
            package.version = Some(
                Version::parse(&version_str)
                    .map_err(|e| RezCoreError::PackageParse(format!("Invalid version: {}", e)))?,
            );
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
    // Function definitions for late binding
    functions: HashMap<String, String>,
}

impl PackageData {
    fn new() -> Self {
        Self::default()
    }
}

impl PythonAstParser {
    /// Evaluate a constant value
    fn evaluate_constant(&self, constant: &Constant) -> Result<PythonValue, RezCoreError> {
        match constant {
            Constant::Str(s) => Ok(PythonValue::String(s.clone())),
            Constant::Int(i) => {
                let int_val = i
                    .to_string()
                    .parse::<i64>()
                    .map_err(|_| RezCoreError::PackageParse("Integer too large".to_string()))?;
                Ok(PythonValue::Integer(int_val))
            }
            Constant::Float(f) => Ok(PythonValue::Float(*f)),
            Constant::Bool(b) => Ok(PythonValue::Boolean(*b)),
            Constant::None => Ok(PythonValue::None),
            _ => Err(RezCoreError::PackageParse(format!(
                "Unsupported constant: {:?}",
                constant
            ))),
        }
    }

    /// Evaluate binary operations
    fn evaluate_binary_operation(
        &self,
        binop: &rustpython_ast::ExprBinOp,
    ) -> Result<PythonValue, RezCoreError> {
        let left = self.evaluate_expression(&binop.left)?;
        let right = self.evaluate_expression(&binop.right)?;

        match (&left, &binop.op, &right) {
            (PythonValue::String(l), Operator::Add, PythonValue::String(r)) => {
                Ok(PythonValue::String(format!("{}{}", l, r)))
            }
            (PythonValue::Integer(l), Operator::Add, PythonValue::Integer(r)) => {
                Ok(PythonValue::Integer(l + r))
            }
            (PythonValue::Float(l), Operator::Add, PythonValue::Float(r)) => {
                Ok(PythonValue::Float(l + r))
            }
            (PythonValue::List(l), Operator::Add, PythonValue::List(r)) => {
                let mut result = l.clone();
                result.extend(r.clone());
                Ok(PythonValue::List(result))
            }
            (PythonValue::Integer(l), Operator::Sub, PythonValue::Integer(r)) => {
                Ok(PythonValue::Integer(l - r))
            }
            (PythonValue::Integer(l), Operator::Mult, PythonValue::Integer(r)) => {
                Ok(PythonValue::Integer(l * r))
            }
            (PythonValue::String(l), Operator::Mult, PythonValue::Integer(r)) => {
                Ok(PythonValue::String(l.repeat(*r as usize)))
            }
            _ => Ok(PythonValue::Expression(format!(
                "{:?} {:?} {:?}",
                left, binop.op, right
            ))),
        }
    }

    /// Evaluate unary operations
    fn evaluate_unary_operation(
        &self,
        unaryop: &rustpython_ast::ExprUnaryOp,
    ) -> Result<PythonValue, RezCoreError> {
        let operand = self.evaluate_expression(&unaryop.operand)?;

        match (&unaryop.op, &operand) {
            (UnaryOp::Not, PythonValue::Boolean(b)) => Ok(PythonValue::Boolean(!b)),
            (UnaryOp::UAdd, PythonValue::Integer(i)) => Ok(PythonValue::Integer(*i)),
            (UnaryOp::USub, PythonValue::Integer(i)) => Ok(PythonValue::Integer(-i)),
            (UnaryOp::UAdd, PythonValue::Float(f)) => Ok(PythonValue::Float(*f)),
            (UnaryOp::USub, PythonValue::Float(f)) => Ok(PythonValue::Float(-f)),
            _ => Ok(PythonValue::Expression(format!(
                "{:?} {:?}",
                unaryop.op, operand
            ))),
        }
    }

    /// Evaluate comparison operations
    fn evaluate_comparison(
        &self,
        compare: &rustpython_ast::ExprCompare,
    ) -> Result<PythonValue, RezCoreError> {
        let left = self.evaluate_expression(&compare.left)?;

        if compare.ops.len() != compare.comparators.len() {
            return Ok(PythonValue::Expression(format!("{:?}", compare)));
        }

        for (op, comparator) in compare.ops.iter().zip(compare.comparators.iter()) {
            let right = self.evaluate_expression(comparator)?;

            let result = match (&left, op, &right) {
                (PythonValue::Integer(l), CmpOp::Eq, PythonValue::Integer(r)) => *l == *r,
                (PythonValue::String(l), CmpOp::Eq, PythonValue::String(r)) => l == r,
                (PythonValue::Boolean(l), CmpOp::Eq, PythonValue::Boolean(r)) => l == r,
                (PythonValue::Integer(l), CmpOp::Lt, PythonValue::Integer(r)) => l < r,
                (PythonValue::Integer(l), CmpOp::Gt, PythonValue::Integer(r)) => l > r,
                _ => {
                    return Ok(PythonValue::Expression(format!(
                        "{:?} {:?} {:?}",
                        left, op, right
                    )))
                }
            };

            if !result {
                return Ok(PythonValue::Boolean(false));
            }
        }

        Ok(PythonValue::Boolean(true))
    }

    /// Evaluate boolean operations
    fn evaluate_boolean_operation(
        &self,
        boolop: &rustpython_ast::ExprBoolOp,
    ) -> Result<PythonValue, RezCoreError> {
        match &boolop.op {
            BoolOp::And => {
                for value in &boolop.values {
                    let result = self.evaluate_expression(value)?;
                    if let PythonValue::Boolean(false) = result {
                        return Ok(PythonValue::Boolean(false));
                    }
                }
                Ok(PythonValue::Boolean(true))
            }
            BoolOp::Or => {
                for value in &boolop.values {
                    let result = self.evaluate_expression(value)?;
                    if let PythonValue::Boolean(true) = result {
                        return Ok(PythonValue::Boolean(true));
                    }
                }
                Ok(PythonValue::Boolean(false))
            }
        }
    }

    /// Evaluate list expressions
    fn evaluate_list(&self, list: &rustpython_ast::ExprList) -> Result<PythonValue, RezCoreError> {
        let mut result = Vec::new();
        for elt in &list.elts {
            result.push(self.evaluate_expression(elt)?);
        }
        Ok(PythonValue::List(result))
    }

    /// Evaluate tuple expressions
    fn evaluate_tuple(
        &self,
        tuple: &rustpython_ast::ExprTuple,
    ) -> Result<PythonValue, RezCoreError> {
        let mut result = Vec::new();
        for elt in &tuple.elts {
            result.push(self.evaluate_expression(elt)?);
        }
        Ok(PythonValue::List(result)) // Treat tuples as lists for simplicity
    }

    /// Evaluate dictionary expressions
    fn evaluate_dict(&self, dict: &rustpython_ast::ExprDict) -> Result<PythonValue, RezCoreError> {
        let mut result = HashMap::new();
        for (key, value) in dict.keys.iter().zip(dict.values.iter()) {
            if let Some(key) = key {
                let key_val = self.evaluate_expression(key)?;
                let value_val = self.evaluate_expression(value)?;

                if let PythonValue::String(key_str) = key_val {
                    result.insert(key_str, value_val);
                } else {
                    return Err(RezCoreError::PackageParse(
                        "Dictionary keys must be strings".to_string(),
                    ));
                }
            }
        }
        Ok(PythonValue::Dict(result))
    }

    /// Evaluate function calls
    fn evaluate_function_call(
        &self,
        call: &rustpython_ast::ExprCall,
    ) -> Result<PythonValue, RezCoreError> {
        // For now, return as expression string
        // In a full implementation, we'd handle built-in functions
        Ok(PythonValue::Expression(format!("{:?}", call)))
    }

    /// Evaluate attribute access
    fn evaluate_attribute(
        &self,
        attr: &rustpython_ast::ExprAttribute,
    ) -> Result<PythonValue, RezCoreError> {
        // For now, return as expression string
        // In a full implementation, we'd handle module attributes
        Ok(PythonValue::Expression(format!("{:?}", attr)))
    }

    /// Evaluate subscript operations
    fn evaluate_subscript(
        &self,
        subscript: &rustpython_ast::ExprSubscript,
    ) -> Result<PythonValue, RezCoreError> {
        // For now, return as expression string
        // In a full implementation, we'd handle list/dict indexing
        Ok(PythonValue::Expression(format!("{:?}", subscript)))
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
        // The parser should handle both branches of the conditional
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
        // The parser should handle expression evaluation
    }
}
