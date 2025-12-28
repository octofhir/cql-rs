//! Evaluator benchmarks using divan
//!
//! Benchmarks for CQL expression evaluation performance.

use octofhir_cql_elm::{BinaryExpression, Element, Expression, ListExpression, Literal, UnaryExpression};
use octofhir_cql_eval::{CqlEngine, EvaluationContext};

fn main() {
    divan::main();
}

// Helper to create an empty element
fn empty_element() -> Element {
    Element {
        locator: None,
        result_type_name: None,
        result_type_specifier: None,
    }
}

// Helper to create a literal expression
fn literal_int(value: i64) -> Expression {
    Expression::Literal(Literal {
        element: Element {
            locator: None,
            result_type_name: Some("Integer".to_string()),
            result_type_specifier: None,
        },
        value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
        value: Some(value.to_string()),
    })
}

fn literal_decimal(value: &str) -> Expression {
    Expression::Literal(Literal {
        element: Element {
            locator: None,
            result_type_name: Some("Decimal".to_string()),
            result_type_specifier: None,
        },
        value_type: "{urn:hl7-org:elm-types:r1}Decimal".to_string(),
        value: Some(value.to_string()),
    })
}

fn literal_string(value: &str) -> Expression {
    Expression::Literal(Literal {
        element: Element {
            locator: None,
            result_type_name: Some("String".to_string()),
            result_type_specifier: None,
        },
        value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
        value: Some(value.to_string()),
    })
}

fn literal_boolean(value: bool) -> Expression {
    Expression::Literal(Literal {
        element: Element {
            locator: None,
            result_type_name: Some("Boolean".to_string()),
            result_type_specifier: None,
        },
        value_type: "{urn:hl7-org:elm-types:r1}Boolean".to_string(),
        value: Some(value.to_string()),
    })
}

// === Literal Evaluation Benchmarks ===

mod literals {
    use super::*;

    #[divan::bench]
    fn integer_literal(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = literal_int(42);

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench]
    fn decimal_literal(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = literal_decimal("3.14159");

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench]
    fn string_literal(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = literal_string("Hello, World!");

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench]
    fn boolean_literal(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = literal_boolean(true);

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }
}

// === Arithmetic Operation Benchmarks ===

mod arithmetic {
    use super::*;

    #[divan::bench]
    fn simple_addition(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = Expression::Add(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(literal_int(1)), Box::new(literal_int(2))],
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench]
    fn complex_arithmetic(bencher: divan::Bencher) {
        let engine = CqlEngine::new();

        // (1 + 2) * 3 - 4 / 2
        let add = Expression::Add(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(literal_int(1)), Box::new(literal_int(2))],
        });

        let mult = Expression::Multiply(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(add), Box::new(literal_int(3))],
        });

        let div = Expression::Divide(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(literal_int(4)), Box::new(literal_int(2))],
        });

        let expr = Expression::Subtract(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(mult), Box::new(div)],
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }
}

// === Comparison Operation Benchmarks ===

mod comparisons {
    use super::*;

    #[divan::bench]
    fn integer_comparison(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = Expression::Greater(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(literal_int(10)), Box::new(literal_int(5))],
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench]
    fn equality(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = Expression::Equal(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(literal_int(42)), Box::new(literal_int(42))],
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }
}

// === Logical Operation Benchmarks ===

mod logical {
    use super::*;

    #[divan::bench]
    fn and_operation(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = Expression::And(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(literal_boolean(true)), Box::new(literal_boolean(false))],
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench]
    fn or_operation(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = Expression::Or(BinaryExpression {
            element: empty_element(),
            operand: vec![Box::new(literal_boolean(true)), Box::new(literal_boolean(false))],
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench]
    fn not_operation(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = Expression::Not(UnaryExpression {
            element: empty_element(),
            operand: Box::new(literal_boolean(true)),
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }
}

// === List Operation Benchmarks ===

mod lists {
    use super::*;

    #[divan::bench]
    fn small_list_creation(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let expr = Expression::List(ListExpression {
            element: empty_element(),
            type_specifier: None,
            elements: Some(vec![
                Box::new(literal_int(1)),
                Box::new(literal_int(2)),
                Box::new(literal_int(3)),
                Box::new(literal_int(4)),
                Box::new(literal_int(5)),
            ]),
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench]
    fn large_list_creation(bencher: divan::Bencher) {
        let engine = CqlEngine::new();
        let elements: Vec<Box<Expression>> = (1..=100).map(|i| Box::new(literal_int(i))).collect();
        let expr = Expression::List(ListExpression {
            element: empty_element(),
            type_specifier: None,
            elements: Some(elements),
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }
}

// === Scaling Benchmarks ===

mod scaling {
    use super::*;

    #[divan::bench(args = [10, 50, 100, 200])]
    fn arithmetic_chain_scaling(bencher: divan::Bencher, n: usize) {
        let engine = CqlEngine::new();

        let mut expr = literal_int(1);
        for _ in 1..n {
            expr = Expression::Add(BinaryExpression {
                element: empty_element(),
                operand: vec![Box::new(expr), Box::new(literal_int(1))],
            });
        }

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }

    #[divan::bench(args = [10, 50, 100, 500, 1000])]
    fn list_size_scaling(bencher: divan::Bencher, n: usize) {
        let engine = CqlEngine::new();
        let elements: Vec<Box<Expression>> = (1..=n as i64).map(|i| Box::new(literal_int(i))).collect();
        let expr = Expression::List(ListExpression {
            element: empty_element(),
            type_specifier: None,
            elements: Some(elements),
        });

        bencher.bench_local(|| {
            let mut ctx = EvaluationContext::new();
            engine.evaluate(divan::black_box(&expr), &mut ctx)
        });
    }
}
