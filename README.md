# octofhir-cql

A high-performance implementation of Clinical Quality Language (CQL) 1.5 in Rust.

`octofhir-cql` provides a complete toolchain for clinical logic execution, from parsing human-readable CQL to efficient evaluation against FHIR and other data sources.

## Features

- **Complete CQL 1.5 Support**: Robust parser and execution engine.
- **ELM Support**: Full Expression Logical Model (ELM) generation and evaluation.
- **High Performance**: Built with Rust for speed and memory safety.
- **Pluggable Data Sources**: Easily integrate with FHIR servers, databases, or in-memory data.
- **Three-Valued Logic**: Correct implementation of CQL null handling.
- **Advanced Queries**: Full support for `retrieve`, `where`, `sort`, `aggregate`, and multi-source queries.

## Quick Start

Add `octofhir-cql` to your `Cargo.toml`:

```toml
[dependencies]
octofhir-cql = "0.1.0"
```

### Basic Usage

Evaluate a simple CQL expression:

```rust
use octofhir_cql::{parse, elm::AstToElmConverter, eval::{CqlEngine, EvaluationContext}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cql = r#"
        library Example version '1.0.0'
        
        define InPopulation:
            AgeInYears() >= 18
            
        define Result:
            1 + 1
    "#;

    // 1. Parse CQL to AST
    let ast = parse(cql)?;

    // 2. Convert AST to ELM
    let mut converter = AstToElmConverter::new();
    let elm = converter.convert_library(&ast);

    // 3. Set up evaluation context
    let engine = CqlEngine::new();
    let mut ctx = EvaluationContext::new().with_library(elm.clone());

    // 4. Evaluate a specific expression
    let result = engine.evaluate_expression(&elm, "Result", &mut ctx)?;

    println!("Result: {:?}", result); // CqlValue::Integer(2)
    
    Ok(())
}
```

## Advanced Usage

### Data Providers

To evaluate clinical expressions that use `retrieve`, you need to provide a `DataProvider`. You can implement the `DataRetriever` trait from `octofhir-cql-model`.

```rust
use octofhir_cql::model::{DataRetriever, DataRetrieverError};
use octofhir_cql::eval::retrieve::DataRetrieverAdapter;
use octofhir_cql_types::{CqlValue, CqlCode, CqlInterval};
use std::sync::Arc;
use async_trait::async_trait;

struct MyRetriever;

#[async_trait]
impl DataRetriever for MyRetriever {
    async fn retrieve(
        &self,
        context: &str,
        data_type: &str,
        code_property: Option<&str>,
        codes: Option<&[CqlCode]>,
        template_id: Option<&str>,
        date_property: Option<&str>,
        date_range: Option<&CqlInterval>,
    ) -> Result<Vec<CqlValue>, DataRetrieverError> {
        // Implement your data fetching logic here (e.g., call a FHIR API)
        Ok(vec![])
    }
}

// Usage in context
let retriever = Arc::new(MyRetriever);
let adapter = DataRetrieverAdapter::new(retriever, "Patient");
let mut ctx = EvaluationContextBuilder::new()
    .data_provider(Arc::new(adapter))
    .build();
```

### Terminology Providers

Similarly, for `InValueSet` or `InCodeSystem` operators, you can implement a `TerminologyProvider`. The library provides a `TerminologyAdapter` that can be wrapped around any implementation of the `TerminologyRetriever` trait.

## CLI

The project includes a command-line interface for testing and analyzing CQL.

```bash
# Evaluate an expression directly
cargo run --bin cql -- eval "1 + 1"

# Parse a CQL file to ELM
cargo run --bin cql -- parse measurements.cql --format elm
```

## Project Structure

- `octofhir-cql`: Main entry point and re-exports.
- `octofhir-cql-parser`: Winnow-based CQL parser.
- `octofhir-cql-ast`: Abstract Syntax Tree definitions.
- `octofhir-cql-elm`: ELM model and AST-to-ELM converter.
- `octofhir-cql-eval`: Evaluation engine and built-in operators.
- `octofhir-cql-model`: Domain models and provider traits.
- `octofhir-cql-types`: Core implementation of CQL types and values.

## License

This project is licensed under the Apache License 2.0 or MIT License, at your option.
