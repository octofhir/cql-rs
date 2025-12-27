//! Clinical Operators for CQL
//!
//! Implements clinical-specific operations:
//! - CalculateAge, CalculateAgeAt
//! - InValueSet, InCodeSystem
//! - ExpandValueSet
//! - Code/Concept literals
//! - Quantity/Ratio expressions

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use octofhir_cql_elm::{
    CalculateAgeAtExpression, CalculateAgeExpression, CodeLiteralExpression, ConceptLiteralExpression,
    InCodeSystemExpression, InValueSetExpression, QuantityExpression, RatioExpression,
    DateTimePrecision as ElmPrecision,
};
use octofhir_cql_types::{CqlCode, CqlConcept, CqlQuantity, CqlRatio, CqlValue};

/// Convert ELM DateTimePrecision to the internal precision type for age calculation
fn elm_precision_to_age_unit(precision: &ElmPrecision) -> AgeUnit {
    match precision {
        ElmPrecision::Year => AgeUnit::Year,
        ElmPrecision::Month => AgeUnit::Month,
        ElmPrecision::Week => AgeUnit::Week,
        ElmPrecision::Day => AgeUnit::Day,
        ElmPrecision::Hour => AgeUnit::Hour,
        ElmPrecision::Minute => AgeUnit::Minute,
        ElmPrecision::Second => AgeUnit::Second,
        ElmPrecision::Millisecond => AgeUnit::Millisecond,
    }
}

/// Age calculation unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgeUnit {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

impl CqlEngine {
    /// Evaluate a Code literal expression
    ///
    /// Creates a Code value from the literal expression
    pub fn eval_code_literal(
        &self,
        expr: &CodeLiteralExpression,
        _ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let code = CqlCode {
            code: expr.code.clone(),
            system: expr.system.name.clone(),
            version: expr.version.clone(),
            display: expr.display.clone(),
        };

        Ok(CqlValue::Code(code))
    }

    /// Evaluate a Concept literal expression
    ///
    /// Creates a Concept value from the literal expression
    pub fn eval_concept_literal(
        &self,
        expr: &ConceptLiteralExpression,
        _ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let codes: Vec<CqlCode> = expr
            .code
            .iter()
            .map(|c| CqlCode {
                code: c.code.clone(),
                system: c.system.name.clone(),
                version: c.version.clone(),
                display: c.display.clone(),
            })
            .collect();

        let concept = CqlConcept {
            codes: codes.into(),
            display: expr.display.clone(),
        };

        Ok(CqlValue::Concept(concept))
    }

    /// Evaluate a Quantity expression
    pub fn eval_quantity(&self, expr: &QuantityExpression) -> EvalResult<CqlValue> {
        let value = expr.value.ok_or_else(|| {
            EvalError::internal("Quantity expression missing value")
        })?;

        Ok(CqlValue::Quantity(CqlQuantity {
            value,
            unit: expr.unit.clone(),
        }))
    }

    /// Evaluate a Ratio expression
    pub fn eval_ratio(
        &self,
        expr: &RatioExpression,
        _ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let numerator = CqlQuantity {
            value: expr.numerator.value.ok_or_else(|| {
                EvalError::internal("Ratio numerator missing value")
            })?,
            unit: expr.numerator.unit.clone(),
        };

        let denominator = CqlQuantity {
            value: expr.denominator.value.ok_or_else(|| {
                EvalError::internal("Ratio denominator missing value")
            })?,
            unit: expr.denominator.unit.clone(),
        };

        Ok(CqlValue::Ratio(CqlRatio {
            numerator,
            denominator,
        }))
    }

    /// Evaluate InCodeSystem expression
    ///
    /// Returns true if the code is in the specified code system
    pub fn eval_in_code_system(
        &self,
        expr: &InCodeSystemExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let code_value = self.evaluate(&expr.code, ctx)?;

        if code_value.is_null() {
            return Ok(CqlValue::Null);
        }

        // Get code system ID
        let code_system_id = if let Some(cs_expr) = &expr.codesystem_expression {
            let cs = self.evaluate(cs_expr, ctx)?;
            match cs {
                CqlValue::String(s) => s,
                _ => return Err(EvalError::type_mismatch("String", cs.get_type().name())),
            }
        } else {
            expr.codesystem.name.clone()
        };

        // Use terminology provider
        if let Some(provider) = ctx.terminology_provider() {
            let result = provider.in_code_system(&code_value, &code_system_id);
            match result {
                Some(b) => Ok(CqlValue::Boolean(b)),
                None => Ok(CqlValue::Null),
            }
        } else {
            // Without terminology provider, we can only check if the code's system matches
            match &code_value {
                CqlValue::Code(code) => {
                    Ok(CqlValue::Boolean(&code.system == &code_system_id))
                }
                CqlValue::Concept(concept) => {
                    // Check if any code in the concept is in the code system
                    let any_match = concept.codes.iter().any(|c| &c.system == &code_system_id);
                    Ok(CqlValue::Boolean(any_match))
                }
                CqlValue::String(_) => {
                    // String codes can't be verified without a terminology provider
                    Ok(CqlValue::Null)
                }
                _ => Err(EvalError::type_mismatch("Code, Concept, or String", code_value.get_type().name())),
            }
        }
    }

    /// Evaluate InValueSet expression
    ///
    /// Returns true if the code is in the specified value set
    pub fn eval_in_value_set(
        &self,
        expr: &InValueSetExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let code_value = self.evaluate(&expr.code, ctx)?;

        if code_value.is_null() {
            return Ok(CqlValue::Null);
        }

        // Get value set ID
        let value_set_id = if let Some(vs_expr) = &expr.valueset_expression {
            let vs = self.evaluate(vs_expr, ctx)?;
            match vs {
                CqlValue::String(s) => s,
                CqlValue::Tuple(t) => {
                    // Value set reference tuple
                    t.get("name")
                        .and_then(|v| match v {
                            CqlValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .ok_or_else(|| EvalError::internal("Invalid value set reference"))?
                }
                _ => return Err(EvalError::type_mismatch("String or ValueSet", vs.get_type().name())),
            }
        } else if let Some(vs_ref) = &expr.valueset {
            vs_ref.name.clone()
        } else {
            return Err(EvalError::internal("InValueSet expression missing value set"));
        };

        // Use terminology provider
        if let Some(provider) = ctx.terminology_provider() {
            let result = provider.in_value_set(&code_value, &value_set_id);
            match result {
                Some(b) => Ok(CqlValue::Boolean(b)),
                None => Ok(CqlValue::Null),
            }
        } else {
            // Without terminology provider, we can't verify value set membership
            Ok(CqlValue::Null)
        }
    }

    /// Evaluate CalculateAge expression
    ///
    /// Calculates the age at the evaluation timestamp from a birthdate
    pub fn eval_calculate_age(
        &self,
        expr: &CalculateAgeExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let birthdate = self.evaluate(&expr.operand, ctx)?;

        if birthdate.is_null() {
            return Ok(CqlValue::Null);
        }

        // Get current date
        let now = ctx.now();

        // Convert birthdate to components
        let (birth_year, birth_month, birth_day) = match &birthdate {
            CqlValue::Date(d) => (d.year, d.month, d.day),
            CqlValue::DateTime(dt) => (dt.year, dt.month, dt.day),
            _ => return Err(EvalError::type_mismatch("Date or DateTime", birthdate.get_type().name())),
        };

        // Calculate age based on precision
        let age_unit = elm_precision_to_age_unit(&expr.precision);
        let age = calculate_age_between(
            birth_year as i32,
            birth_month,
            birth_day,
            now.year,
            now.month,
            now.day,
            age_unit,
        );

        Ok(CqlValue::Integer(age))
    }

    /// Evaluate CalculateAgeAt expression
    ///
    /// Calculates the age at a specific date from a birthdate
    pub fn eval_calculate_age_at(
        &self,
        expr: &CalculateAgeAtExpression,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        if expr.operand.len() != 2 {
            return Err(EvalError::internal("CalculateAgeAt requires exactly 2 operands"));
        }

        let birthdate = self.evaluate(&expr.operand[0], ctx)?;
        let as_of = self.evaluate(&expr.operand[1], ctx)?;

        if birthdate.is_null() || as_of.is_null() {
            return Ok(CqlValue::Null);
        }

        // Convert birthdate to components
        let (birth_year, birth_month, birth_day) = match &birthdate {
            CqlValue::Date(d) => (d.year, d.month, d.day),
            CqlValue::DateTime(dt) => (dt.year, dt.month, dt.day),
            _ => return Err(EvalError::type_mismatch("Date or DateTime", birthdate.get_type().name())),
        };

        // Convert as_of to components
        let (as_of_year, as_of_month, as_of_day) = match &as_of {
            CqlValue::Date(d) => (d.year as i32, d.month, d.day),
            CqlValue::DateTime(dt) => (dt.year, dt.month, dt.day),
            _ => return Err(EvalError::type_mismatch("Date or DateTime", as_of.get_type().name())),
        };

        // Calculate age based on precision
        let age_unit = elm_precision_to_age_unit(&expr.precision);
        let age = calculate_age_between(
            birth_year as i32,
            birth_month,
            birth_day,
            as_of_year,
            as_of_month,
            as_of_day,
            age_unit,
        );

        Ok(CqlValue::Integer(age))
    }
}

/// Calculate age between two dates at a given precision
fn calculate_age_between(
    birth_year: i32,
    birth_month: Option<u8>,
    birth_day: Option<u8>,
    as_of_year: i32,
    as_of_month: Option<u8>,
    as_of_day: Option<u8>,
    precision: AgeUnit,
) -> i32 {
    let birth_month = birth_month.unwrap_or(1);
    let birth_day = birth_day.unwrap_or(1);
    let as_of_month = as_of_month.unwrap_or(1);
    let as_of_day = as_of_day.unwrap_or(1);

    match precision {
        AgeUnit::Year => {
            let mut years = as_of_year - birth_year;
            // Adjust if birthday hasn't occurred yet this year
            if (as_of_month, as_of_day) < (birth_month, birth_day) {
                years -= 1;
            }
            years
        }
        AgeUnit::Month => {
            let mut months = (as_of_year - birth_year) * 12 + (as_of_month as i32 - birth_month as i32);
            // Adjust if birthday hasn't occurred yet this month
            if as_of_day < birth_day {
                months -= 1;
            }
            months
        }
        AgeUnit::Week => {
            // Calculate total days and divide by 7
            let total_days = days_between(
                birth_year,
                birth_month,
                birth_day,
                as_of_year,
                as_of_month,
                as_of_day,
            );
            total_days / 7
        }
        AgeUnit::Day => {
            days_between(
                birth_year,
                birth_month,
                birth_day,
                as_of_year,
                as_of_month,
                as_of_day,
            )
        }
        AgeUnit::Hour => {
            // For date-only values, we use midnight
            days_between(
                birth_year,
                birth_month,
                birth_day,
                as_of_year,
                as_of_month,
                as_of_day,
            ) * 24
        }
        AgeUnit::Minute => {
            days_between(
                birth_year,
                birth_month,
                birth_day,
                as_of_year,
                as_of_month,
                as_of_day,
            ) * 24 * 60
        }
        AgeUnit::Second => {
            days_between(
                birth_year,
                birth_month,
                birth_day,
                as_of_year,
                as_of_month,
                as_of_day,
            ) * 24 * 60 * 60
        }
        AgeUnit::Millisecond => {
            // This would overflow i32 for most realistic ages, but we follow the spec
            days_between(
                birth_year,
                birth_month,
                birth_day,
                as_of_year,
                as_of_month,
                as_of_day,
            ) * 24 * 60 * 60 * 1000
        }
    }
}

/// Calculate the number of days between two dates
fn days_between(
    year1: i32,
    month1: u8,
    day1: u8,
    year2: i32,
    month2: u8,
    day2: u8,
) -> i32 {
    // Use Julian day numbers for accurate calculation
    let jd1 = julian_day_number(year1, month1, day1);
    let jd2 = julian_day_number(year2, month2, day2);
    jd2 - jd1
}

/// Calculate Julian day number for a date
fn julian_day_number(year: i32, month: u8, day: u8) -> i32 {
    let a = (14 - month as i32) / 12;
    let y = year + 4800 - a;
    let m = month as i32 + 12 * a - 3;

    // Gregorian calendar calculation
    day as i32 + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045
}

/// Check if a Code is equivalent to another Code value
pub fn codes_equivalent(code1: &CqlCode, code2: &CqlCode) -> bool {
    // Codes are equivalent if code and system match
    code1.code == code2.code && code1.system == code2.system
}

/// Check if a Code is in a list of Codes
pub fn code_in_codes(code: &CqlCode, codes: &[CqlCode]) -> bool {
    codes.iter().any(|c| codes_equivalent(code, c))
}

/// Check if any Code in a Concept is in a list of Codes
pub fn concept_in_codes(concept: &CqlConcept, codes: &[CqlCode]) -> bool {
    concept.codes.iter().any(|c| code_in_codes(c, codes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use octofhir_cql_elm::{CodeSystemRef, Element};

    fn engine() -> CqlEngine {
        CqlEngine::new()
    }

    fn ctx() -> EvaluationContext {
        EvaluationContext::new()
    }

    #[test]
    fn test_code_literal() {
        let e = engine();
        let mut c = ctx();

        let expr = CodeLiteralExpression {
            element: Element::default(),
            system: CodeSystemRef {
                element: Element::default(),
                name: "http://loinc.org".to_string(),
                library_name: None,
            },
            code: "8480-6".to_string(),
            display: Some("Systolic blood pressure".to_string()),
            version: None,
        };

        let result = e.eval_code_literal(&expr, &mut c).unwrap();
        match result {
            CqlValue::Code(code) => {
                assert_eq!(code.code, "8480-6");
                assert_eq!(code.system, "http://loinc.org");
                assert_eq!(code.display, Some("Systolic blood pressure".to_string()));
            }
            _ => panic!("Expected Code"),
        }
    }

    #[test]
    fn test_concept_literal() {
        let e = engine();
        let mut c = ctx();

        let expr = ConceptLiteralExpression {
            element: Element::default(),
            code: vec![
                CodeLiteralExpression {
                    element: Element::default(),
                    system: CodeSystemRef {
                        element: Element::default(),
                        name: "http://loinc.org".to_string(),
                        library_name: None,
                    },
                    code: "8480-6".to_string(),
                    display: None,
                    version: None,
                },
                CodeLiteralExpression {
                    element: Element::default(),
                    system: CodeSystemRef {
                        element: Element::default(),
                        name: "http://snomed.info/sct".to_string(),
                        library_name: None,
                    },
                    code: "271649006".to_string(),
                    display: None,
                    version: None,
                },
            ],
            display: Some("Blood pressure".to_string()),
        };

        let result = e.eval_concept_literal(&expr, &mut c).unwrap();
        match result {
            CqlValue::Concept(concept) => {
                assert_eq!(concept.codes.len(), 2);
                assert_eq!(concept.display, Some("Blood pressure".to_string()));
            }
            _ => panic!("Expected Concept"),
        }
    }

    #[test]
    fn test_quantity_expression() {
        let e = engine();

        let expr = QuantityExpression {
            element: Element::default(),
            value: Some(rust_decimal::Decimal::new(120, 0)),
            unit: Some("mmHg".to_string()),
        };

        let result = e.eval_quantity(&expr).unwrap();
        match result {
            CqlValue::Quantity(q) => {
                assert_eq!(q.value, rust_decimal::Decimal::new(120, 0));
                assert_eq!(q.unit, Some("mmHg".to_string()));
            }
            _ => panic!("Expected Quantity"),
        }
    }

    #[test]
    fn test_calculate_age_years() {
        // Test the age calculation function directly
        let age = calculate_age_between(
            1990, Some(6), Some(15),  // Birth: June 15, 1990
            2024, Some(12), Some(27), // As of: December 27, 2024
            AgeUnit::Year,
        );
        assert_eq!(age, 34);

        // Birthday not yet occurred this year
        let age = calculate_age_between(
            1990, Some(6), Some(15),  // Birth: June 15, 1990
            2024, Some(3), Some(1),   // As of: March 1, 2024
            AgeUnit::Year,
        );
        assert_eq!(age, 33);
    }

    #[test]
    fn test_calculate_age_months() {
        let age = calculate_age_between(
            1990, Some(6), Some(15),
            1991, Some(8), Some(20),
            AgeUnit::Month,
        );
        assert_eq!(age, 14); // 14 months

        // Day of month not reached
        let age = calculate_age_between(
            1990, Some(6), Some(15),
            1991, Some(8), Some(10),
            AgeUnit::Month,
        );
        assert_eq!(age, 13); // 13 months (birthday not reached in August)
    }

    #[test]
    fn test_calculate_age_days() {
        let age = calculate_age_between(
            2024, Some(1), Some(1),
            2024, Some(1), Some(31),
            AgeUnit::Day,
        );
        assert_eq!(age, 30); // 30 days
    }

    #[test]
    fn test_julian_day_number() {
        // Test known dates
        // January 1, 2000 should be JD 2451545
        let jd = julian_day_number(2000, 1, 1);
        assert_eq!(jd, 2451545);
    }

    #[test]
    fn test_codes_equivalent() {
        let code1 = CqlCode {
            code: "8480-6".to_string(),
            system: "http://loinc.org".to_string(),
            version: None,
            display: None,
        };

        let code2 = CqlCode {
            code: "8480-6".to_string(),
            system: "http://loinc.org".to_string(),
            version: None,
            display: Some("Different display".to_string()),
        };

        let code3 = CqlCode {
            code: "8480-6".to_string(),
            system: "http://snomed.info/sct".to_string(),
            version: None,
            display: None,
        };

        assert!(codes_equivalent(&code1, &code2)); // Same code and system
        assert!(!codes_equivalent(&code1, &code3)); // Different system
    }
}
