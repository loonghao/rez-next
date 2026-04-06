//! Expression evaluation logic for the Python AST parser.

use super::types::PythonValue;
use super::PythonAstParser;
use rez_next_common::RezCoreError;
use rustpython_ast::{BoolOp, CmpOp, Constant, Expr, Operator, UnaryOp};
use std::collections::HashMap;

impl PythonAstParser {
    /// Evaluate a Python expression to a value
    pub(super) fn evaluate_expression(&self, expr: &Expr) -> Result<PythonValue, RezCoreError> {
        match expr {
            Expr::Constant(constant) => self.evaluate_constant(&constant.value),
            Expr::Name(name) => {
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
            _ => Ok(PythonValue::Expression(format!("{:?}", expr))),
        }
    }

    /// Evaluate a constant value
    pub(super) fn evaluate_constant(
        &self,
        constant: &Constant,
    ) -> Result<PythonValue, RezCoreError> {
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
    fn evaluate_list(
        &self,
        list: &rustpython_ast::ExprList,
    ) -> Result<PythonValue, RezCoreError> {
        let mut result = Vec::new();
        for elt in &list.elts {
            result.push(self.evaluate_expression(elt)?);
        }
        Ok(PythonValue::List(result))
    }

    /// Evaluate tuple expressions (treated as lists)
    fn evaluate_tuple(
        &self,
        tuple: &rustpython_ast::ExprTuple,
    ) -> Result<PythonValue, RezCoreError> {
        let mut result = Vec::new();
        for elt in &tuple.elts {
            result.push(self.evaluate_expression(elt)?);
        }
        Ok(PythonValue::List(result))
    }

    /// Evaluate dictionary expressions
    fn evaluate_dict(
        &self,
        dict: &rustpython_ast::ExprDict,
    ) -> Result<PythonValue, RezCoreError> {
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

    /// Evaluate function calls (returns expression string for unsupported calls)
    fn evaluate_function_call(
        &self,
        call: &rustpython_ast::ExprCall,
    ) -> Result<PythonValue, RezCoreError> {
        Ok(PythonValue::Expression(format!("{:?}", call)))
    }

    /// Evaluate attribute access (returns expression string)
    fn evaluate_attribute(
        &self,
        attr: &rustpython_ast::ExprAttribute,
    ) -> Result<PythonValue, RezCoreError> {
        Ok(PythonValue::Expression(format!("{:?}", attr)))
    }

    /// Evaluate subscript operations (returns expression string)
    fn evaluate_subscript(
        &self,
        subscript: &rustpython_ast::ExprSubscript,
    ) -> Result<PythonValue, RezCoreError> {
        Ok(PythonValue::Expression(format!("{:?}", subscript)))
    }
}
