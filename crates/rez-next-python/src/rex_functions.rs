//! Rex command language functions exposed to Python.

use pyo3::prelude::*;

/// Interpret a Rex command string and return resulting environment variables.
/// Equivalent to `rez.rex.interpret(commands, executor=...)`
#[pyfunction]
#[pyo3(signature = (commands, vars=None))]
pub fn rex_interpret(
    py: Python,
    commands: &str,
    vars: Option<std::collections::HashMap<String, String>>,
) -> PyResult<Py<PyAny>> {
    use rez_next_rex::RexExecutor;

    let mut executor = RexExecutor::new();
    if let Some(context_vars) = vars {
        for (k, v) in context_vars {
            executor.set_context_var(k, v);
        }
    }
    let env = executor
        .execute_commands(commands, "", None, None)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let dict = pyo3::types::PyDict::new(py);
    for (k, v) in &env.vars {
        dict.set_item(k, v)?;
    }
    Ok(dict.into_any().unbind())
}
