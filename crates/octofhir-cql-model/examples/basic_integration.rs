//! Basic integration example showing how to use the CQL data model components
//!
//! This example demonstrates:
//! - Loading FHIR ModelInfo (R4/R5)
//! - Using ModelProvider for type lookups
//! - Creating a DataRetriever
//! - Using TerminologyProvider

use octofhir_cql_model::{
    fhir::{fhir_r4_registry, fhir_r5_registry},
    DataRetriever, ModelProvider, NoOpDataRetriever,
};
use octofhir_fhir_model::NoOpTerminologyProvider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== CQL Data Model Integration Example ===\n");

    // 1. Load FHIR R4 ModelInfo
    println!("1. Loading FHIR R4 ModelInfo...");
    let r4_registry = fhir_r4_registry()?;
    println!("   ✓ FHIR R4 ModelInfo loaded successfully");

    // 2. Query type information
    println!("\n2. Querying Patient type information...");
    if let Some(patient_type) = r4_registry.get_type("Patient").await? {
        println!("   ✓ Found Patient type");
        println!("     - Retrievable: {}", patient_type.retrievable);
        println!(
            "     - Primary code path: {}",
            patient_type.primary_code_path.as_deref().unwrap_or("N/A")
        );
        println!("     - Properties: {}", patient_type.elements.len());
    }

    // 3. Query property information
    println!("\n3. Querying Patient.id property...");
    if let Some(id_prop) = r4_registry.get_property_type("Patient", "id").await? {
        println!("   ✓ Found id property");
        println!("     - Type: {}", id_prop.element_type);
        println!("     - Is List: {}", id_prop.is_list);
    }

    // 4. Load FHIR R5 ModelInfo
    println!("\n4. Loading FHIR R5 ModelInfo...");
    let r5_registry = fhir_r5_registry()?;
    println!("   ✓ FHIR R5 ModelInfo loaded successfully");

    // 5. Create a DataRetriever
    println!("\n5. Creating DataRetriever...");
    let retriever = Arc::new(NoOpDataRetriever::new()) as Arc<dyn DataRetriever>;
    println!("   ✓ DataRetriever created");

    // 6. Create a TerminologyProvider
    println!("\n6. Creating TerminologyProvider...");
    let _terminology = Arc::new(NoOpTerminologyProvider);
    println!("   ✓ TerminologyProvider created");

    println!("\n=== Integration Complete ===");
    println!("All components are working together successfully!");

    Ok(())
}
