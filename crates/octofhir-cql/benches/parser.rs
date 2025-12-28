//! Parser benchmarks using divan
//!
//! Benchmarks for CQL parser performance across various input types and sizes.

use octofhir_cql_parser::{parse, parse_expression};

fn main() {
    divan::main();
}

// === Simple Expression Benchmarks ===

mod literals {
    use super::*;

    #[divan::bench]
    fn integer_literal(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("42")));
    }

    #[divan::bench]
    fn decimal_literal(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("3.14159")));
    }

    #[divan::bench]
    fn string_literal(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("'Hello, World!'")));
    }

    #[divan::bench]
    fn boolean_literal(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("true")));
    }

    #[divan::bench]
    fn date_literal(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("@2024-03-15")));
    }

    #[divan::bench]
    fn datetime_literal(bencher: divan::Bencher) {
        bencher.bench_local(|| {
            parse_expression(divan::black_box("@2024-03-15T10:30:00.123Z"))
        });
    }
}

// === Arithmetic Expression Benchmarks ===

mod arithmetic {
    use super::*;

    #[divan::bench]
    fn simple_addition(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("1 + 2")));
    }

    #[divan::bench]
    fn complex_arithmetic(bencher: divan::Bencher) {
        bencher.bench_local(|| {
            parse_expression(divan::black_box("(1 + 2) * 3 - 4 / 2 + 5 ^ 2"))
        });
    }

    #[divan::bench]
    fn nested_arithmetic(bencher: divan::Bencher) {
        bencher.bench_local(|| {
            parse_expression(divan::black_box(
                "((1 + 2) * (3 - 4)) / ((5 + 6) * (7 - 8))",
            ))
        });
    }
}

// === Logical Expression Benchmarks ===

mod logical {
    use super::*;

    #[divan::bench]
    fn simple_logical(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("true and false")));
    }

    #[divan::bench]
    fn complex_logical(bencher: divan::Bencher) {
        bencher.bench_local(|| {
            parse_expression(divan::black_box(
                "(x > 5 and y < 10) or (z = 15 and w != 20)",
            ))
        });
    }
}

// === Query Benchmarks ===

mod queries {
    use super::*;

    #[divan::bench]
    fn simple_query(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("[Observation] O")));
    }

    #[divan::bench]
    fn query_with_where(bencher: divan::Bencher) {
        bencher.bench_local(|| {
            parse_expression(divan::black_box("[Observation] O where O.status = 'final'"))
        });
    }

    #[divan::bench]
    fn complex_query(bencher: divan::Bencher) {
        let query = "[Observation] O
                 where O.status = 'final' and O.value > 120
                 return Tuple { code: O.code, value: O.value }
                 sort by O.effectiveDateTime desc";
        bencher.bench_local(|| parse_expression(divan::black_box(query)));
    }
}

// === List and Tuple Benchmarks ===

mod collections {
    use super::*;

    #[divan::bench]
    fn small_list(bencher: divan::Bencher) {
        bencher.bench_local(|| parse_expression(divan::black_box("{1, 2, 3, 4, 5}")));
    }

    #[divan::bench]
    fn large_list(bencher: divan::Bencher) {
        let list_expr = format!(
            "{{{}}}",
            (1..=100)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        bencher
            .with_inputs(|| list_expr.clone())
            .bench_local_values(|expr| parse_expression(divan::black_box(&expr)));
    }

    #[divan::bench]
    fn tuple(bencher: divan::Bencher) {
        bencher.bench_local(|| {
            parse_expression(divan::black_box(
                "Tuple { id: '123', value: 42, active: true }",
            ))
        });
    }
}

// === Library Benchmarks ===

mod libraries {
    use super::*;

    #[divan::bench]
    fn minimal_library(bencher: divan::Bencher) {
        bencher
            .bench_local(|| parse(divan::black_box("library Test version '1.0.0'")));
    }

    #[divan::bench]
    fn simple_library(bencher: divan::Bencher) {
        let lib = "library Test version '1.0.0'
                 define \"Value\": 42
                 define \"Result\": \"Value\" * 2";
        bencher.bench_local(|| parse(divan::black_box(lib)));
    }

    #[divan::bench]
    fn realistic_library(bencher: divan::Bencher) {
        let library = r#"
        library CMS146 version '2.0.0'

        using FHIR version '4.0.1'

        include FHIRHelpers version '4.0.1' called FH

        codesystem "LOINC": 'http://loinc.org'
        codesystem "SNOMED": 'http://snomed.info/sct'

        valueset "Pharyngitis": 'http://cts.nlm.nih.gov/fhir/ValueSet/pharyngitis'

        code "Strep Test": '6557-3' from "LOINC"

        parameter "Measurement Period" Interval<DateTime>

        context Patient

        define "In Demographic":
          AgeInYearsAt(start of "Measurement Period") >= 2
            and AgeInYearsAt(start of "Measurement Period") < 18

        define "Pharyngitis Encounters":
          [Encounter: "Pharyngitis"] E
            where E.period during "Measurement Period"
              and E.status = 'finished'

        define "Antibiotic Prescribed":
          [MedicationRequest: "Antibiotic Medications"] M
            where M.authoredOn during "Measurement Period"

        define "Numerator":
          "Pharyngitis Encounters" E
            with "Antibiotic Prescribed" A
              such that A.authoredOn 3 days or less after start of E.period
    "#;
        bencher.bench_local(|| parse(divan::black_box(library)));
    }
}

// === Scaling Benchmarks ===

mod scaling {
    use super::*;

    #[divan::bench(args = [10, 50, 100, 200, 500])]
    fn expression_scaling(bencher: divan::Bencher, n: usize) {
        let expr = (0..n)
            .map(|i| format!("{}", i))
            .collect::<Vec<_>>()
            .join(" + ");

        bencher
            .with_inputs(|| expr.clone())
            .bench_local_values(|e| parse_expression(divan::black_box(&e)));
    }

    #[divan::bench(args = [10, 50, 100, 500, 1000])]
    fn list_scaling(bencher: divan::Bencher, n: usize) {
        let list_expr = format!(
            "{{{}}}",
            (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
        );

        bencher
            .with_inputs(|| list_expr.clone())
            .bench_local_values(|e| parse_expression(divan::black_box(&e)));
    }
}
