//! Logical Operators for CQL
//!
//! Implements: And, Or, Xor, Implies, Not, IsNull, IsTrue, IsFalse, Coalesce, If, Case
//! All logical operators implement three-valued logic per CQL specification

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use octofhir_cql_elm::{BinaryExpression, CaseExpression, IfExpression, NaryExpression, UnaryExpression};
use octofhir_cql_types::CqlValue;

impl CqlEngine {
    /// Evaluate And operator with three-valued logic
    ///
    /// Truth table:
    /// | A     | B     | A and B |
    /// |-------|-------|---------|
    /// | true  | true  | true    |
    /// | true  | false | false   |
    /// | true  | null  | null    |
    /// | false | true  | false   |
    /// | false | false | false   |
    /// | false | null  | false   |
    /// | null  | true  | null    |
    /// | null  | false | false   |
    /// | null  | null  | null    |
    pub fn eval_and(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        match (&left, &right) {
            // If either is false, result is false
            (CqlValue::Boolean(false), _) | (_, CqlValue::Boolean(false)) => {
                Ok(CqlValue::Boolean(false))
            }
            // Both true -> true
            (CqlValue::Boolean(true), CqlValue::Boolean(true)) => {
                Ok(CqlValue::Boolean(true))
            }
            // Any null with non-false -> null
            (CqlValue::Null, _) | (_, CqlValue::Null) => {
                Ok(CqlValue::Null)
            }
            // Type error if not boolean
            _ => Err(EvalError::type_mismatch("Boolean", left.get_type().name())),
        }
    }

    /// Evaluate Or operator with three-valued logic
    ///
    /// Truth table:
    /// | A     | B     | A or B  |
    /// |-------|-------|---------|
    /// | true  | true  | true    |
    /// | true  | false | true    |
    /// | true  | null  | true    |
    /// | false | true  | true    |
    /// | false | false | false   |
    /// | false | null  | null    |
    /// | null  | true  | true    |
    /// | null  | false | null    |
    /// | null  | null  | null    |
    pub fn eval_or(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        match (&left, &right) {
            // If either is true, result is true
            (CqlValue::Boolean(true), _) | (_, CqlValue::Boolean(true)) => {
                Ok(CqlValue::Boolean(true))
            }
            // Both false -> false
            (CqlValue::Boolean(false), CqlValue::Boolean(false)) => {
                Ok(CqlValue::Boolean(false))
            }
            // Any null with non-true -> null
            (CqlValue::Null, _) | (_, CqlValue::Null) => {
                Ok(CqlValue::Null)
            }
            // Type error if not boolean
            _ => Err(EvalError::type_mismatch("Boolean", left.get_type().name())),
        }
    }

    /// Evaluate Xor (exclusive or) operator
    ///
    /// Returns true if exactly one operand is true
    pub fn eval_xor(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        if left.is_null() || right.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&left, &right) {
            (CqlValue::Boolean(a), CqlValue::Boolean(b)) => {
                Ok(CqlValue::Boolean(*a != *b))
            }
            _ => Err(EvalError::type_mismatch("Boolean", left.get_type().name())),
        }
    }

    /// Evaluate Implies operator
    ///
    /// A implies B is equivalent to (not A) or B
    /// Right-associative
    pub fn eval_implies(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (left, right) = self.eval_binary_operands(expr, ctx)?;

        match (&left, &right) {
            // false implies anything -> true
            (CqlValue::Boolean(false), _) => Ok(CqlValue::Boolean(true)),
            // true implies B -> B
            (CqlValue::Boolean(true), CqlValue::Boolean(b)) => Ok(CqlValue::Boolean(*b)),
            (CqlValue::Boolean(true), CqlValue::Null) => Ok(CqlValue::Null),
            // null implies true -> true
            (CqlValue::Null, CqlValue::Boolean(true)) => Ok(CqlValue::Boolean(true)),
            // null implies false -> null, null implies null -> null
            (CqlValue::Null, _) => Ok(CqlValue::Null),
            _ => Err(EvalError::type_mismatch("Boolean", left.get_type().name())),
        }
    }

    /// Evaluate Not operator
    ///
    /// not true -> false
    /// not false -> true
    /// not null -> null
    pub fn eval_not(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Boolean(b) => Ok(CqlValue::Boolean(!b)),
            CqlValue::Null => Ok(CqlValue::Null),
            _ => Err(EvalError::type_mismatch("Boolean", operand.get_type().name())),
        }
    }

    /// Evaluate IsNull operator
    ///
    /// Returns true if operand is null, false otherwise
    /// Never returns null
    pub fn eval_is_null(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        Ok(CqlValue::Boolean(operand.is_null()))
    }

    /// Evaluate IsTrue operator
    ///
    /// Returns true if operand is exactly true, false otherwise
    /// Never returns null
    pub fn eval_is_true(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        Ok(CqlValue::Boolean(operand.is_true()))
    }

    /// Evaluate IsFalse operator
    ///
    /// Returns true if operand is exactly false, false otherwise
    /// Never returns null
    pub fn eval_is_false(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;
        Ok(CqlValue::Boolean(operand.is_false()))
    }

    /// Evaluate Coalesce operator
    ///
    /// Returns the first non-null value in the list
    /// If all values are null, returns null
    pub fn eval_coalesce(&self, expr: &NaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        for operand in &expr.operand {
            let value = self.evaluate(operand, ctx)?;
            if !value.is_null() {
                return Ok(value);
            }
        }
        Ok(CqlValue::Null)
    }

    /// Evaluate If expression
    ///
    /// if condition then thenExpr else elseExpr
    pub fn eval_if(&self, expr: &IfExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let condition = self.evaluate(&expr.condition, ctx)?;

        match &condition {
            CqlValue::Boolean(true) => self.evaluate(&expr.then, ctx),
            CqlValue::Boolean(false) | CqlValue::Null => self.evaluate(&expr.else_clause, ctx),
            _ => Err(EvalError::type_mismatch("Boolean", condition.get_type().name())),
        }
    }

    /// Evaluate Case expression
    ///
    /// Supports both:
    /// - case when cond1 then expr1 when cond2 then expr2 else elseExpr end
    /// - case comparand when val1 then expr1 when val2 then expr2 else elseExpr end
    pub fn eval_case(&self, expr: &CaseExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        // If there's a comparand, compare each case item to it
        if let Some(comparand_expr) = &expr.comparand {
            let comparand = self.evaluate(comparand_expr, ctx)?;

            for case_item in &expr.case_item {
                let when_value = self.evaluate(&case_item.when, ctx)?;

                // Use equality comparison
                let equal = match (&comparand, &when_value) {
                    (CqlValue::Null, _) | (_, CqlValue::Null) => false,
                    _ => crate::operators::comparison::cql_equal(&comparand, &when_value)?,
                };

                if equal {
                    return self.evaluate(&case_item.then, ctx);
                }
            }
        } else {
            // No comparand - each when clause is a boolean condition
            for case_item in &expr.case_item {
                let condition = self.evaluate(&case_item.when, ctx)?;

                if condition.is_true() {
                    return self.evaluate(&case_item.then, ctx);
                }
            }
        }

        // No case matched - return else clause or null
        if let Some(else_expr) = &expr.else_clause {
            self.evaluate(else_expr, ctx)
        } else {
            Ok(CqlValue::Null)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use octofhir_cql_elm::{Element, Expression, Literal, NullLiteral};

    fn engine() -> CqlEngine {
        CqlEngine::new()
    }

    fn ctx() -> EvaluationContext {
        EvaluationContext::new()
    }

    fn bool_expr(value: bool) -> Box<Expression> {
        Box::new(Expression::Literal(Literal {
            element: Element::default(),
            value_type: "{urn:hl7-org:elm-types:r1}Boolean".to_string(),
            value: Some(value.to_string()),
        }))
    }

    fn null_expr() -> Box<Expression> {
        Box::new(Expression::Null(NullLiteral { element: Element::default() }))
    }

    fn make_binary(left: Box<Expression>, right: Box<Expression>) -> BinaryExpression {
        BinaryExpression {
            element: Element::default(),
            operand: vec![left, right],
        }
    }

    fn make_unary(operand: Box<Expression>) -> UnaryExpression {
        UnaryExpression {
            element: Element::default(),
            operand,
        }
    }

    #[test]
    fn test_and_true_true() {
        let e = engine();
        let mut c = ctx();
        let expr = make_binary(bool_expr(true), bool_expr(true));
        let result = e.eval_and(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));
    }

    #[test]
    fn test_and_true_false() {
        let e = engine();
        let mut c = ctx();
        let expr = make_binary(bool_expr(true), bool_expr(false));
        let result = e.eval_and(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(false));
    }

    #[test]
    fn test_and_null_false() {
        let e = engine();
        let mut c = ctx();
        let expr = make_binary(null_expr(), bool_expr(false));
        let result = e.eval_and(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(false));
    }

    #[test]
    fn test_and_true_null() {
        let e = engine();
        let mut c = ctx();
        let expr = make_binary(bool_expr(true), null_expr());
        let result = e.eval_and(&expr, &mut c).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_or_true_false() {
        let e = engine();
        let mut c = ctx();
        let expr = make_binary(bool_expr(true), bool_expr(false));
        let result = e.eval_or(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));
    }

    #[test]
    fn test_or_false_null() {
        let e = engine();
        let mut c = ctx();
        let expr = make_binary(bool_expr(false), null_expr());
        let result = e.eval_or(&expr, &mut c).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_not_true() {
        let e = engine();
        let mut c = ctx();
        let expr = make_unary(bool_expr(true));
        let result = e.eval_not(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(false));
    }

    #[test]
    fn test_not_null() {
        let e = engine();
        let mut c = ctx();
        let expr = make_unary(null_expr());
        let result = e.eval_not(&expr, &mut c).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_is_null() {
        let e = engine();
        let mut c = ctx();

        let expr = make_unary(null_expr());
        let result = e.eval_is_null(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));

        let expr = make_unary(bool_expr(true));
        let result = e.eval_is_null(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(false));
    }

    #[test]
    fn test_implies() {
        let e = engine();
        let mut c = ctx();

        // false implies anything -> true
        let expr = make_binary(bool_expr(false), bool_expr(false));
        let result = e.eval_implies(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));

        // true implies true -> true
        let expr = make_binary(bool_expr(true), bool_expr(true));
        let result = e.eval_implies(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));

        // true implies false -> false
        let expr = make_binary(bool_expr(true), bool_expr(false));
        let result = e.eval_implies(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(false));
    }

    #[test]
    fn test_xor() {
        let e = engine();
        let mut c = ctx();

        // true xor false -> true
        let expr = make_binary(bool_expr(true), bool_expr(false));
        let result = e.eval_xor(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));

        // true xor true -> false
        let expr = make_binary(bool_expr(true), bool_expr(true));
        let result = e.eval_xor(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(false));
    }

    #[test]
    fn test_coalesce() {
        let e = engine();
        let mut c = ctx();

        let expr = NaryExpression {
            element: Element::default(),
            operand: vec![null_expr(), null_expr(), bool_expr(true), bool_expr(false)],
        };
        let result = e.eval_coalesce(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));
    }

    #[test]
    fn test_if() {
        let e = engine();
        let mut c = ctx();

        let expr = IfExpression {
            element: Element::default(),
            condition: bool_expr(true),
            then: Box::new(Expression::Literal(Literal {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
                value: Some("1".to_string()),
            })),
            else_clause: Box::new(Expression::Literal(Literal {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
                value: Some("2".to_string()),
            })),
        };

        let result = e.eval_if(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Integer(1));
    }
}
