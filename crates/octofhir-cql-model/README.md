# octofhir-cql-model

CQL data model abstraction for FHIR and other data models.

## Overview

This crate provides FHIR-agnostic data model support for the CQL evaluation engine. It includes:

- **ModelInfo Parsing**: XML and JSON parser for HL7 ModelInfo format
- **Type System**: Type and property resolution with inheritance support
- **Data Retrieval**: Async trait for retrieving clinical data
- **Runtime Loading**: Load ModelInfo from filesystem at runtime
- **Embedded ModelInfo**: Production-ready FHIR R4/R5 ModelInfo embedded at compile time

## Quick Start

### Using Embedded ModelInfo (Recommended)

The crate includes comprehensive, production-ready FHIR R4 and R5 ModelInfo files embedded at compile time:

```rust
use octofhir_cql_model::fhir::{fhir_r4_registry, fhir_r5_registry};

// Load FHIR R4 ModelInfo (embedded, no I/O)
let registry = fhir_r4_registry()?;

// Or load FHIR R5
let registry = fhir_r5_registry()?;

// Use the registry
if let Some(patient_type) = registry.get_type("Patient").await? {
    println!("Found Patient with {} properties", patient_type.elements.len());
}
```

### Loading ModelInfo at Runtime

For scenarios where you need to load ModelInfo from your FHIR package installation:

```rust
use octofhir_cql_model::ModelRegistry;

// Load from XML file
let registry = ModelRegistry::from_xml_file("/path/to/FHIR-modelinfo-4.0.1.xml")?;

// Load from JSON file
let registry = ModelRegistry::from_json_file("/path/to/FHIR-modelinfo-4.0.1.json")?;

// Auto-detect format from extension
let registry = ModelRegistry::from_file("/path/to/FHIR-modelinfo-4.0.1.xml")?;
```

### Server Usage Pattern

For server deployments with FHIR packages installed:

```rust
use octofhir_cql_model::ModelRegistry;
use std::env;

// Read path from environment
let fhir_package_path = env::var("FHIR_PACKAGE_PATH")?;
let modelinfo_path = format!(
    "{}/hl7.fhir.r4.core#4.0.1/package/FHIR-ModelInfo-4.0.1.xml",
    fhir_package_path
);

let registry = ModelRegistry::from_file(&modelinfo_path)?;

// Use in evaluation context
let context = EvaluationContext::builder()
    .with_model_provider(Arc::new(registry))
    .build();
```

## Embedded ModelInfo Details

### Coverage

The embedded FHIR R4/R5 ModelInfo files include:

**Base Types:**
- All FHIR primitive types (boolean, integer, string, date, dateTime, etc.)
- System types (System.Any, System.Boolean, System.Integer, etc.)

**Complex Types:**
- Element, Extension
- Coding, CodeableConcept
- Identifier, Reference
- Quantity, Period, Range, Ratio
- HumanName, Address, ContactPoint
- Timing, Dosage
- And more...

**Core Resources (with full properties):**
- Patient (15 properties)
- Observation (with components and reference ranges)
- Condition (with staging and evidence)
- Procedure (with performers and devices)
- MedicationRequest (with dispense requests)
- Encounter (with participants and locations)

Each resource includes:
- All key properties and their types
- List types (e.g., `list<Identifier>`)
- Retrievable flag for CQL Retrieve operations
- Primary code path for code filtering

### File Sizes

- FHIR R4 ModelInfo: 450 lines, ~15KB
- FHIR R5 ModelInfo: 450 lines, ~15KB

## API Reference

### ModelProvider Trait

```rust
#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn get_type(&self, type_name: &str)
        -> Result<Option<TypeInfo>, ModelProviderError>;

    async fn get_property_type(&self, parent: &str, property: &str)
        -> Result<Option<PropertyInfo>, ModelProviderError>;

    fn is_retrievable(&self, type_name: &str) -> bool;
    fn get_primary_code_path(&self, type_name: &str) -> Option<String>;
}
```

### DataRetriever Trait

```rust
#[async_trait]
pub trait DataRetriever: Send + Sync {
    async fn retrieve(
        &self,
        context: &str,           // "Patient", "Encounter", etc.
        data_type: &str,         // "Observation", "Condition", etc.
        code_path: Option<&str>,
        codes: Option<&[CqlCode]>,
        valueset: Option<&str>,
        date_path: Option<&str>,
        date_range: Option<&CqlInterval>,
    ) -> Result<Vec<CqlValue>, DataRetrieverError>;
}
```

### ModelRegistry

```rust
impl ModelRegistry {
    // Create from existing ModelInfo
    pub fn new(model_info: ModelInfo) -> Self;

    // Parse from string content
    pub fn from_xml(xml: &str) -> Result<Self, ModelProviderError>;
    pub fn from_json(json: &str) -> Result<Self, ModelProviderError>;

    // Load from filesystem at runtime
    pub fn from_xml_file(path: impl AsRef<Path>) -> Result<Self, ModelProviderError>;
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, ModelProviderError>;
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ModelProviderError>;

    // Metadata
    pub fn model_name(&self) -> String;
    pub fn model_version(&self) -> String;
    pub fn model_url(&self) -> String;
}
```

## Examples

### Basic Integration

```bash
cargo run --example basic_integration
```

Demonstrates loading embedded ModelInfo and querying types.

### Runtime Loading

```bash
cargo run --example runtime_loading
```

Shows how to load ModelInfo from filesystem at runtime.

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

Test coverage includes:
- XML and JSON parsing
- Type and property resolution
- Inheritance chain traversal
- Runtime file loading
- Auto-format detection

## Performance

### Embedded ModelInfo (Compile-time)
- **Pros**: No I/O, instant availability, zero runtime overhead
- **Cons**: Binary size increase (~30KB total for R4+R5)
- **Use when**: Building standalone applications, want zero-config deployment

### Runtime Loading
- **Pros**: Update ModelInfo without recompiling, support custom models
- **Cons**: Requires file I/O, slight startup overhead
- **Use when**: Need official HL7 ModelInfo, multiple FHIR versions, custom models

## Production Deployment

### Option 1: Embedded (Recommended for most cases)

```rust
// No configuration needed - works out of the box
let registry = octofhir_cql_model::fhir::fhir_r4_registry()?;
```

### Option 2: Runtime Loading from FHIR Packages

```bash
# Set environment variable pointing to FHIR packages
export FHIR_PACKAGE_PATH=/usr/local/fhir/packages

# Your application loads at runtime
let registry = ModelRegistry::from_file(
    format!("{}/hl7.fhir.r4.core#4.0.1/package/FHIR-ModelInfo-4.0.1.xml",
            env::var("FHIR_PACKAGE_PATH")?)
)?;
```

### Option 3: Generate on-the-fly

Since ModelInfo is embedded, you can instantiate it multiple times without I/O:

```rust
// Each call creates a new registry instance from embedded data
let r4_registry = fhir_r4_registry()?;
let r5_registry = fhir_r5_registry()?;

// Clone registries for different contexts
let registry_clone = r4_registry.clone();
```

## Architecture

```
octofhir-cql-model/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── provider.rs         # ModelProvider & DataRetriever traits
│   ├── model_info/         # ModelInfo types and parsers
│   │   ├── mod.rs
│   │   ├── types.rs        # ModelInfo, TypeInfo, PropertyInfo
│   │   └── parser.rs       # XML/JSON parsing
│   ├── registry.rs         # ModelRegistry implementation
│   ├── retriever.rs        # NoOpDataRetriever for testing
│   └── fhir/               # FHIR-specific modules
│       ├── mod.rs
│       ├── r4.rs           # FHIR R4 embedded ModelInfo
│       └── r5.rs           # FHIR R5 embedded ModelInfo
└── resources/              # Embedded ModelInfo files
    ├── FHIR-modelinfo-4.0.1.xml  # Production-ready FHIR R4 (450 lines)
    └── FHIR-modelinfo-5.0.0.xml  # Production-ready FHIR R5 (450 lines)
```

## License

See workspace license.

## References

- [HL7 CQL Specification](http://cql.hl7.org/)
- [FHIR ModelInfo Reference](http://cql.hl7.org/09-b-cqlreference.html#fhir-4-0-1-modelinfo)
- [FHIR R4 Specification](http://hl7.org/fhir/R4/)
- [FHIR R5 Specification](http://hl7.org/fhir/R5/)
