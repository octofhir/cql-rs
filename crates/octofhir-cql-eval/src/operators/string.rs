//! String Operators for CQL
//!
//! Implements: Concatenate, Combine, Split, SplitOnMatches, Length, Upper, Lower,
//! Substring, PositionOf, LastPositionOf, StartsWith, EndsWith, Matches, ReplaceMatches,
//! Indexer (for strings)

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use octofhir_cql_elm::{
    BinaryExpression, CombineExpression, LastPositionOfExpression, NaryExpression,
    PositionOfExpression, SplitExpression, SplitOnMatchesExpression, SubstringExpression,
    TernaryExpression, UnaryExpression,
};
use octofhir_cql_types::{CqlList, CqlType, CqlValue};
use regex::Regex;

impl CqlEngine {
    /// Evaluate Concatenate operator
    ///
    /// Concatenates all string operands. Returns null if any operand is null.
    pub fn eval_concatenate(&self, expr: &NaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let mut result = String::new();

        for operand in &expr.operand {
            let value = self.evaluate(operand, ctx)?;
            match value {
                CqlValue::Null => return Ok(CqlValue::Null),
                CqlValue::String(s) => result.push_str(&s),
                _ => return Err(EvalError::type_mismatch("String", value.get_type().name())),
            }
        }

        Ok(CqlValue::String(result))
    }

    /// Evaluate Combine operator
    ///
    /// Combines a list of strings with an optional separator.
    /// Null values in the list are ignored.
    pub fn eval_combine(&self, expr: &CombineExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let source = self.evaluate(&expr.source, ctx)?;

        if source.is_null() {
            return Ok(CqlValue::Null);
        }

        let list = match source {
            CqlValue::List(l) => l,
            _ => return Err(EvalError::type_mismatch("List<String>", source.get_type().name())),
        };

        // Get separator (default to empty string)
        let separator = if let Some(sep_expr) = &expr.separator {
            match self.evaluate(sep_expr, ctx)? {
                CqlValue::Null => return Ok(CqlValue::Null),
                CqlValue::String(s) => s,
                other => return Err(EvalError::type_mismatch("String", other.get_type().name())),
            }
        } else {
            String::new()
        };

        // Combine non-null strings
        let strings: Vec<String> = list
            .iter()
            .filter_map(|v| match v {
                CqlValue::String(s) => Some(s.clone()),
                CqlValue::Null => None,
                _ => None,
            })
            .collect();

        if strings.is_empty() {
            Ok(CqlValue::Null)
        } else {
            Ok(CqlValue::String(strings.join(&separator)))
        }
    }

    /// Evaluate Split operator
    ///
    /// Splits a string by a separator into a list of strings.
    /// If separator is null, returns a list with the original string.
    pub fn eval_split(&self, expr: &SplitExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let string_to_split = self.evaluate(&expr.string_to_split, ctx)?;

        if string_to_split.is_null() {
            return Ok(CqlValue::Null);
        }

        let s = match &string_to_split {
            CqlValue::String(s) => s.clone(),
            _ => return Err(EvalError::type_mismatch("String", string_to_split.get_type().name())),
        };

        let separator = if let Some(sep_expr) = &expr.separator {
            match self.evaluate(sep_expr, ctx)? {
                CqlValue::Null => {
                    // Per CQL spec, if separator is null, return list with the original string
                    return Ok(CqlValue::List(CqlList {
                        element_type: CqlType::String,
                        elements: vec![CqlValue::string(&s)],
                    }));
                }
                CqlValue::String(s) => s,
                other => return Err(EvalError::type_mismatch("String", other.get_type().name())),
            }
        } else {
            String::new() // Default to empty separator if not provided
        };

        let parts: Vec<CqlValue> = s.split(separator.as_str()).map(|p| CqlValue::string(p)).collect();

        Ok(CqlValue::List(CqlList {
            element_type: CqlType::String,
            elements: parts,
        }))
    }

    /// Evaluate SplitOnMatches operator
    ///
    /// Splits a string by a regex pattern.
    pub fn eval_split_on_matches(
        &self,
        expr: &SplitOnMatchesExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let string_to_split = self.evaluate(&expr.string_to_split, ctx)?;
        let separator_pattern = self.evaluate(&expr.separator_pattern, ctx)?;

        if string_to_split.is_null() || separator_pattern.is_null() {
            return Ok(CqlValue::Null);
        }

        let s = match &string_to_split {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", string_to_split.get_type().name())),
        };

        let pattern = match &separator_pattern {
            CqlValue::String(p) => p,
            _ => return Err(EvalError::type_mismatch("String", separator_pattern.get_type().name())),
        };

        let regex = Regex::new(pattern)
            .map_err(|_| EvalError::InvalidRegex { pattern: pattern.clone() })?;

        let parts: Vec<CqlValue> = regex.split(s).map(|p| CqlValue::string(p)).collect();

        Ok(CqlValue::List(CqlList {
            element_type: CqlType::String,
            elements: parts,
        }))
    }

    /// Evaluate Length operator for strings
    pub fn eval_string_length(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::String(s) => Ok(CqlValue::Integer(s.chars().count() as i32)),
            CqlValue::List(l) => Ok(CqlValue::Integer(l.len() as i32)),
            _ => Err(EvalError::unsupported_operator("Length", operand.get_type().name())),
        }
    }

    /// Evaluate Upper operator
    pub fn eval_upper(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::String(s) => Ok(CqlValue::String(s.to_uppercase())),
            _ => Err(EvalError::type_mismatch("String", operand.get_type().name())),
        }
    }

    /// Evaluate Lower operator
    pub fn eval_lower(&self, expr: &UnaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let operand = self.evaluate(&expr.operand, ctx)?;

        match &operand {
            CqlValue::Null => Ok(CqlValue::Null),
            CqlValue::String(s) => Ok(CqlValue::String(s.to_lowercase())),
            _ => Err(EvalError::type_mismatch("String", operand.get_type().name())),
        }
    }

    /// Evaluate Indexer operator for strings (0-based indexing)
    pub fn eval_indexer(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (source, index) = self.eval_binary_operands(expr, ctx)?;

        if source.is_null() || index.is_null() {
            return Ok(CqlValue::Null);
        }

        match (&source, &index) {
            (CqlValue::String(s), CqlValue::Integer(i)) => {
                if *i < 0 {
                    return Ok(CqlValue::Null);
                }
                let idx = *i as usize;
                s.chars()
                    .nth(idx)
                    .map(|c| CqlValue::String(c.to_string()))
                    .unwrap_or(CqlValue::Null)
                    .pipe(Ok)
            }
            (CqlValue::List(l), CqlValue::Integer(i)) => {
                if *i < 0 {
                    return Ok(CqlValue::Null);
                }
                let idx = *i as usize;
                Ok(l.get(idx).cloned().unwrap_or(CqlValue::Null))
            }
            _ => Err(EvalError::unsupported_operator(
                "Indexer",
                format!("{}, {}", source.get_type().name(), index.get_type().name()),
            )),
        }
    }

    /// Evaluate PositionOf operator
    ///
    /// Returns 0-based position of pattern in string, or -1 if not found.
    pub fn eval_position_of(&self, expr: &PositionOfExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let pattern = self.evaluate(&expr.pattern, ctx)?;
        let string = self.evaluate(&expr.string, ctx)?;

        if pattern.is_null() || string.is_null() {
            return Ok(CqlValue::Null);
        }

        let p = match &pattern {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", pattern.get_type().name())),
        };

        let s = match &string {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", string.get_type().name())),
        };

        match s.find(p.as_str()) {
            Some(pos) => Ok(CqlValue::Integer(pos as i32)),
            None => Ok(CqlValue::Integer(-1)),
        }
    }

    /// Evaluate LastPositionOf operator
    pub fn eval_last_position_of(
        &self,
        expr: &LastPositionOfExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let pattern = self.evaluate(&expr.pattern, ctx)?;
        let string = self.evaluate(&expr.string, ctx)?;

        if pattern.is_null() || string.is_null() {
            return Ok(CqlValue::Null);
        }

        let p = match &pattern {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", pattern.get_type().name())),
        };

        let s = match &string {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", string.get_type().name())),
        };

        match s.rfind(p.as_str()) {
            Some(pos) => Ok(CqlValue::Integer(pos as i32)),
            None => Ok(CqlValue::Integer(-1)),
        }
    }

    /// Evaluate Substring operator
    ///
    /// Extracts a substring starting at startIndex (0-based) with optional length.
    pub fn eval_substring(&self, expr: &SubstringExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let string_to_sub = self.evaluate(&expr.string_to_sub, ctx)?;
        let start_index = self.evaluate(&expr.start_index, ctx)?;

        if string_to_sub.is_null() || start_index.is_null() {
            return Ok(CqlValue::Null);
        }

        let s = match &string_to_sub {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", string_to_sub.get_type().name())),
        };

        let start = match &start_index {
            CqlValue::Integer(i) => {
                if *i < 0 {
                    return Ok(CqlValue::Null);
                }
                *i as usize
            }
            _ => return Err(EvalError::type_mismatch("Integer", start_index.get_type().name())),
        };

        let chars: Vec<char> = s.chars().collect();

        if start >= chars.len() {
            return Ok(CqlValue::Null);
        }

        let result = if let Some(length_expr) = &expr.length {
            let length = self.evaluate(length_expr, ctx)?;
            match length {
                CqlValue::Null => return Ok(CqlValue::Null),
                CqlValue::Integer(len) => {
                    if len < 0 {
                        return Ok(CqlValue::Null);
                    }
                    let end = (start + len as usize).min(chars.len());
                    chars[start..end].iter().collect::<String>()
                }
                _ => return Err(EvalError::type_mismatch("Integer", length.get_type().name())),
            }
        } else {
            chars[start..].iter().collect::<String>()
        };

        Ok(CqlValue::String(result))
    }

    /// Evaluate StartsWith operator
    pub fn eval_starts_with(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (string, prefix) = self.eval_binary_operands(expr, ctx)?;

        if string.is_null() || prefix.is_null() {
            return Ok(CqlValue::Null);
        }

        let s = match &string {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", string.get_type().name())),
        };

        let p = match &prefix {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", prefix.get_type().name())),
        };

        Ok(CqlValue::Boolean(s.starts_with(p.as_str())))
    }

    /// Evaluate EndsWith operator
    pub fn eval_ends_with(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (string, suffix) = self.eval_binary_operands(expr, ctx)?;

        if string.is_null() || suffix.is_null() {
            return Ok(CqlValue::Null);
        }

        let s = match &string {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", string.get_type().name())),
        };

        let sfx = match &suffix {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", suffix.get_type().name())),
        };

        Ok(CqlValue::Boolean(s.ends_with(sfx.as_str())))
    }

    /// Evaluate Matches operator
    ///
    /// Returns true if the string matches the regex pattern.
    pub fn eval_matches(&self, expr: &BinaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        let (string, pattern) = self.eval_binary_operands(expr, ctx)?;

        if string.is_null() || pattern.is_null() {
            return Ok(CqlValue::Null);
        }

        let s = match &string {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", string.get_type().name())),
        };

        let p = match &pattern {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", pattern.get_type().name())),
        };

        let regex = Regex::new(p).map_err(|_| EvalError::InvalidRegex { pattern: p.clone() })?;

        Ok(CqlValue::Boolean(regex.is_match(s)))
    }

    /// Evaluate ReplaceMatches operator
    ///
    /// Replaces all occurrences matching the pattern with the substitution.
    pub fn eval_replace_matches(&self, expr: &TernaryExpression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        if expr.operand.len() != 3 {
            return Err(EvalError::internal("ReplaceMatches requires 3 operands"));
        }

        let string = self.evaluate(&expr.operand[0], ctx)?;
        let pattern = self.evaluate(&expr.operand[1], ctx)?;
        let substitution = self.evaluate(&expr.operand[2], ctx)?;

        if string.is_null() || pattern.is_null() || substitution.is_null() {
            return Ok(CqlValue::Null);
        }

        let s = match &string {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", string.get_type().name())),
        };

        let p = match &pattern {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", pattern.get_type().name())),
        };

        let sub = match &substitution {
            CqlValue::String(s) => s,
            _ => return Err(EvalError::type_mismatch("String", substitution.get_type().name())),
        };

        let regex = Regex::new(p).map_err(|_| EvalError::InvalidRegex { pattern: p.clone() })?;

        // Convert CQL/Java-style replacement escapes to Rust regex format
        // In CQL/Java: \$ means literal $, \\ means literal \
        // In Rust regex: $$ means literal $, $ with digit means backreference
        let rust_sub = convert_cql_replacement_to_rust(sub);

        Ok(CqlValue::String(regex.replace_all(s, rust_sub.as_str()).to_string()))
    }
}

/// Convert CQL/Java-style replacement string escapes to Rust regex format
/// - \$ in CQL -> $$ in Rust (literal dollar sign)
/// - \\ in CQL -> \ in Rust (literal backslash)
fn convert_cql_replacement_to_rust(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('$') => {
                    // \$ -> $$ (literal dollar sign)
                    chars.next();
                    result.push_str("$$");
                }
                Some('\\') => {
                    // \\ -> \ (literal backslash)
                    chars.next();
                    result.push('\\');
                }
                _ => {
                    // Other escape sequences pass through
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Trait extension for pipe method
trait Pipe {
    fn pipe<T, F: FnOnce(Self) -> T>(self, f: F) -> T
    where
        Self: Sized;
}

impl<V> Pipe for V {
    fn pipe<T, F: FnOnce(Self) -> T>(self, f: F) -> T {
        f(self)
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

    fn string_expr(s: &str) -> Box<Expression> {
        Box::new(Expression::Literal(Literal {
            element: Element::default(),
            value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
            value: Some(s.to_string()),
        }))
    }

    fn int_expr(i: i32) -> Box<Expression> {
        Box::new(Expression::Literal(Literal {
            element: Element::default(),
            value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
            value: Some(i.to_string()),
        }))
    }

    fn null_expr() -> Box<Expression> {
        Box::new(Expression::Null(NullLiteral { element: Element::default() }))
    }

    #[test]
    fn test_concatenate() {
        let e = engine();
        let mut c = ctx();

        let expr = NaryExpression {
            element: Element::default(),
            operand: vec![string_expr("Hello"), string_expr(", "), string_expr("World!")],
        };

        let result = e.eval_concatenate(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::String("Hello, World!".to_string()));
    }

    #[test]
    fn test_concatenate_with_null() {
        let e = engine();
        let mut c = ctx();

        let expr = NaryExpression {
            element: Element::default(),
            operand: vec![string_expr("Hello"), null_expr()],
        };

        let result = e.eval_concatenate(&expr, &mut c).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_upper() {
        let e = engine();
        let mut c = ctx();

        let expr = UnaryExpression {
            element: Element::default(),
            operand: string_expr("hello"),
        };

        let result = e.eval_upper(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::String("HELLO".to_string()));
    }

    #[test]
    fn test_lower() {
        let e = engine();
        let mut c = ctx();

        let expr = UnaryExpression {
            element: Element::default(),
            operand: string_expr("HELLO"),
        };

        let result = e.eval_lower(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::String("hello".to_string()));
    }

    #[test]
    fn test_length() {
        let e = engine();
        let mut c = ctx();

        let expr = UnaryExpression {
            element: Element::default(),
            operand: string_expr("hello"),
        };

        let result = e.eval_string_length(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Integer(5));
    }

    #[test]
    fn test_starts_with() {
        let e = engine();
        let mut c = ctx();

        let expr = BinaryExpression {
            element: Element::default(),
            operand: vec![string_expr("Hello, World!"), string_expr("Hello")],
        };

        let result = e.eval_starts_with(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));
    }

    #[test]
    fn test_ends_with() {
        let e = engine();
        let mut c = ctx();

        let expr = BinaryExpression {
            element: Element::default(),
            operand: vec![string_expr("Hello, World!"), string_expr("World!")],
        };

        let result = e.eval_ends_with(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));
    }

    #[test]
    fn test_substring() {
        let e = engine();
        let mut c = ctx();

        let expr = SubstringExpression {
            element: Element::default(),
            string_to_sub: string_expr("Hello, World!"),
            start_index: int_expr(7),
            length: Some(int_expr(5)),
        };

        let result = e.eval_substring(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::String("World".to_string()));
    }

    #[test]
    fn test_position_of() {
        let e = engine();
        let mut c = ctx();

        let expr = PositionOfExpression {
            element: Element::default(),
            pattern: string_expr("World"),
            string: string_expr("Hello, World!"),
        };

        let result = e.eval_position_of(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Integer(7));
    }

    #[test]
    fn test_matches() {
        let e = engine();
        let mut c = ctx();

        let expr = BinaryExpression {
            element: Element::default(),
            operand: vec![string_expr("test@example.com"), string_expr(r"^\w+@\w+\.\w+$")],
        };

        let result = e.eval_matches(&expr, &mut c).unwrap();
        assert_eq!(result, CqlValue::Boolean(true));
    }

    #[test]
    fn test_split() {
        let e = engine();
        let mut c = ctx();

        let expr = SplitExpression {
            element: Element::default(),
            string_to_split: string_expr("a,b,c"),
            separator: Some(string_expr(",")),
        };

        let result = e.eval_split(&expr, &mut c).unwrap();
        match result {
            CqlValue::List(l) => {
                assert_eq!(l.len(), 3);
                assert_eq!(l.get(0), Some(&CqlValue::string("a")));
                assert_eq!(l.get(1), Some(&CqlValue::string("b")));
                assert_eq!(l.get(2), Some(&CqlValue::string("c")));
            }
            _ => panic!("Expected list"),
        }
    }
}
