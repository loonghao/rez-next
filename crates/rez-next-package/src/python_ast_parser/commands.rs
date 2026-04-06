//! Processing logic for `commands`, `pre_commands`, and `post_commands` function bodies.
//! Supports the full Rex DSL used by rez package commands.

use super::types::PackageData;
use super::PythonAstParser;
use rez_next_common::RezCoreError;
use rustpython_ast::{Expr, Stmt};

impl PythonAstParser {
    /// Convert a function definition to its string representation
    pub(super) fn function_to_string(
        &self,
        func_def: &rustpython_ast::StmtFunctionDef,
    ) -> Result<String, RezCoreError> {
        Ok(format!("def {}(): ...", func_def.name))
    }

    /// Process the `commands` function body
    pub(super) fn process_commands_function(
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
            package_data.commands_function = Some(commands.join("\n"));
        }
        Ok(())
    }

    /// Process the `pre_commands` function body
    pub(super) fn process_pre_commands_function(
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

    /// Process the `post_commands` function body
    pub(super) fn process_post_commands_function(
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

    /// Extract a Rex DSL command string from a single statement.
    ///
    /// Supports:
    /// - `env.VAR = "value"` (attribute assignment shorthand)
    /// - `env.setenv / prepend_path / append_path / unsetenv / setenv_if_empty`
    /// - `env.VAR.prepend / env.VAR.append / env.VAR.set / env.VAR.unset`
    /// - Top-level: `alias`, `command`, `source`, `info`, `error`, `stop`, `resetenv`
    /// - Top-level shorthand: `setenv`, `prependenv`, `appendenv`, `unsetenv`
    pub(super) fn extract_command_from_statement(
        &self,
        stmt: &Stmt,
    ) -> Result<Option<String>, RezCoreError> {
        match stmt {
            // Handle `env.VAR = "value"` (attribute assignment shorthand)
            Stmt::Assign(assign) => {
                if let Some(Expr::Attribute(attr)) = assign.targets.first() {
                    if let Expr::Name(name_expr) = &*attr.value {
                        if name_expr.id.as_str() == "env" {
                            let var_name = attr.attr.as_str();
                            if let Ok(value) = self.extract_string_value(&assign.value) {
                                return Ok(Some(format!(
                                    "env.setenv('{}', '{}')",
                                    var_name, value
                                )));
                            }
                        }
                    }
                }
            }

            // Handle function calls: env.method(...) and top-level Rex calls
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    if let Expr::Attribute(attr) = &*call.func {
                        // ─── env.method('VAR', ...) ───────────────────────────
                        if let Expr::Name(obj) = &*attr.value {
                            if obj.id.as_str() == "env" {
                                let method = attr.attr.as_str();
                                match method {
                                    "setenv" => {
                                        if call.args.len() >= 2 {
                                            if let (Ok(k), Ok(v)) = (
                                                self.extract_string_value(&call.args[0]),
                                                self.extract_string_value(&call.args[1]),
                                            ) {
                                                return Ok(Some(format!(
                                                    "env.setenv('{}', '{}')",
                                                    k, v
                                                )));
                                            }
                                        }
                                    }
                                    "unsetenv" => {
                                        if let Some(arg) = call.args.first() {
                                            if let Ok(k) = self.extract_string_value(arg) {
                                                return Ok(Some(format!(
                                                    "env.unsetenv('{}')",
                                                    k
                                                )));
                                            }
                                        }
                                    }
                                    "prepend_path" => {
                                        if call.args.len() >= 2 {
                                            if let (Ok(k), Ok(v)) = (
                                                self.extract_string_value(&call.args[0]),
                                                self.extract_string_value(&call.args[1]),
                                            ) {
                                                return Ok(Some(format!(
                                                    "env.prepend_path('{}', '{}')",
                                                    k, v
                                                )));
                                            }
                                        }
                                    }
                                    "append_path" => {
                                        if call.args.len() >= 2 {
                                            if let (Ok(k), Ok(v)) = (
                                                self.extract_string_value(&call.args[0]),
                                                self.extract_string_value(&call.args[1]),
                                            ) {
                                                return Ok(Some(format!(
                                                    "env.append_path('{}', '{}')",
                                                    k, v
                                                )));
                                            }
                                        }
                                    }
                                    "setenv_if_empty" => {
                                        if call.args.len() >= 2 {
                                            if let (Ok(k), Ok(v)) = (
                                                self.extract_string_value(&call.args[0]),
                                                self.extract_string_value(&call.args[1]),
                                            ) {
                                                return Ok(Some(format!(
                                                    "env.setenv_if_empty('{}', '{}')",
                                                    k, v
                                                )));
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // ─── env.VAR.prepend / env.VAR.append ─────────────────
                        if let Expr::Attribute(env_attr) = &*attr.value {
                            if let Expr::Name(obj) = &*env_attr.value {
                                if obj.id.as_str() == "env" {
                                    let var_name = env_attr.attr.as_str();
                                    let method = attr.attr.as_str();
                                    if let Some(arg) = call.args.first() {
                                        if let Ok(value) = self.extract_string_value(arg) {
                                            match method {
                                                "prepend" => {
                                                    return Ok(Some(format!(
                                                        "env.prepend_path('{}', '{}')",
                                                        var_name, value
                                                    )));
                                                }
                                                "append" => {
                                                    return Ok(Some(format!(
                                                        "env.append_path('{}', '{}')",
                                                        var_name, value
                                                    )));
                                                }
                                                "set" => {
                                                    return Ok(Some(format!(
                                                        "env.setenv('{}', '{}')",
                                                        var_name, value
                                                    )));
                                                }
                                                "unset" => {
                                                    return Ok(Some(format!(
                                                        "env.unsetenv('{}')",
                                                        var_name
                                                    )));
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ─── Top-level Rex calls ──────────────────────────────────
                    if let Expr::Name(func_name) = &*call.func {
                        match func_name.id.as_str() {
                            "alias" => {
                                if call.args.len() >= 2 {
                                    if let (Ok(name), Ok(cmd)) = (
                                        self.extract_string_value(&call.args[0]),
                                        self.extract_string_value(&call.args[1]),
                                    ) {
                                        return Ok(Some(format!(
                                            "alias('{}', '{}')",
                                            name, cmd
                                        )));
                                    }
                                }
                            }
                            "command" => {
                                if let Some(arg) = call.args.first() {
                                    if let Ok(cmd) = self.extract_string_value(arg) {
                                        return Ok(Some(format!("command('{}')", cmd)));
                                    }
                                }
                            }
                            "source" => {
                                if let Some(arg) = call.args.first() {
                                    if let Ok(path) = self.extract_string_value(arg) {
                                        return Ok(Some(format!("source('{}')", path)));
                                    }
                                }
                            }
                            "info" => {
                                if let Some(arg) = call.args.first() {
                                    if let Ok(msg) = self.extract_string_value(arg) {
                                        return Ok(Some(format!("info('{}')", msg)));
                                    }
                                }
                            }
                            "error" => {
                                if let Some(arg) = call.args.first() {
                                    if let Ok(msg) = self.extract_string_value(arg) {
                                        return Ok(Some(format!("error('{}')", msg)));
                                    }
                                }
                            }
                            "stop" => {
                                if call.args.is_empty() {
                                    return Ok(Some("stop()".to_string()));
                                } else if let Ok(msg) =
                                    self.extract_string_value(&call.args[0])
                                {
                                    return Ok(Some(format!("stop('{}')", msg)));
                                }
                            }
                            "resetenv" => {
                                if let Some(arg) = call.args.first() {
                                    if let Ok(var) = self.extract_string_value(arg) {
                                        return Ok(Some(format!("resetenv('{}')", var)));
                                    }
                                }
                            }
                            "setenv" => {
                                if call.args.len() >= 2 {
                                    if let (Ok(k), Ok(v)) = (
                                        self.extract_string_value(&call.args[0]),
                                        self.extract_string_value(&call.args[1]),
                                    ) {
                                        return Ok(Some(format!("setenv('{}', '{}')", k, v)));
                                    }
                                }
                            }
                            "prependenv" => {
                                if call.args.len() >= 2 {
                                    if let (Ok(k), Ok(v)) = (
                                        self.extract_string_value(&call.args[0]),
                                        self.extract_string_value(&call.args[1]),
                                    ) {
                                        return Ok(Some(format!(
                                            "prependenv('{}', '{}')",
                                            k, v
                                        )));
                                    }
                                }
                            }
                            "appendenv" => {
                                if call.args.len() >= 2 {
                                    if let (Ok(k), Ok(v)) = (
                                        self.extract_string_value(&call.args[0]),
                                        self.extract_string_value(&call.args[1]),
                                    ) {
                                        return Ok(Some(format!(
                                            "appendenv('{}', '{}')",
                                            k, v
                                        )));
                                    }
                                }
                            }
                            "unsetenv" => {
                                if let Some(arg) = call.args.first() {
                                    if let Ok(k) = self.extract_string_value(arg) {
                                        return Ok(Some(format!("unsetenv('{}')", k)));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }
}
