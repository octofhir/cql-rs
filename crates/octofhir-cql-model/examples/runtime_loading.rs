//! Runtime ModelInfo loading example
//!
//! This example demonstrates how to load FHIR ModelInfo files at runtime
//! from your server's filesystem, rather than using the embedded placeholder files.
//!
//! In production, you would point to the official HL7 FHIR ModelInfo files
//! from your FHIR package installation.

use octofhir_cql_model::{ModelProvider, ModelRegistry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Runtime ModelInfo Loading Example ===\n");

    // Example 1: Load embedded ModelInfo (for testing/development)
    println!("1. Loading embedded FHIR R4 ModelInfo...");
    let embedded_registry = octofhir_cql_model::fhir::fhir_r4_registry()?;
    println!("   ✓ Embedded model loaded: {} v{}",
        embedded_registry.model_name(),
        embedded_registry.model_version()
    );

    // Example 2: Load from runtime file path (recommended for production)
    // This would typically point to your FHIR package installation
    println!("\n2. Loading ModelInfo from filesystem...");

    // Path to the actual ModelInfo file in your codebase
    // In production, this would be something like:
    // "/usr/local/fhir/packages/hl7.fhir.r4.core#4.0.1/package/FHIR-ModelInfo-4.0.1.xml"
    let modelinfo_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/FHIR-modelinfo-4.0.1.xml"
    );

    match ModelRegistry::from_xml_file(modelinfo_path) {
        Ok(registry) => {
            println!("   ✓ Runtime model loaded: {} v{}",
                registry.model_name(),
                registry.model_version()
            );

            // Use the loaded model
            if let Some(patient_type) = registry.get_type("Patient").await? {
                println!("   ✓ Found Patient type with {} properties",
                    patient_type.elements.len()
                );
            }
        }
        Err(e) => {
            println!("   ✗ Failed to load: {}", e);
            println!("   (This is expected if the path doesn't exist on your system)");
        }
    }

    // Example 3: Auto-detect format from extension
    println!("\n3. Auto-detecting file format...");
    match ModelRegistry::from_file(modelinfo_path) {
        Ok(registry) => {
            println!("   ✓ Auto-detected and loaded: {}", registry.model_name());
        }
        Err(e) => {
            println!("   ✗ Failed: {}", e);
        }
    }

    // Example 4: Production usage pattern
    println!("\n4. Production usage pattern:");
    println!("   ```rust");
    println!("   // In your server initialization:");
    println!("   let fhir_package_path = std::env::var(\"FHIR_PACKAGE_PATH\")?;");
    println!("   let modelinfo_path = format!(");
    println!("       \"{{}}/hl7.fhir.r4.core#4.0.1/package/FHIR-ModelInfo-4.0.1.xml\",");
    println!("       fhir_package_path");
    println!("   );");
    println!("   ");
    println!("   let registry = ModelRegistry::from_file(&modelinfo_path)?;");
    println!("   ");
    println!("   // Use the registry in your CQL evaluation context");
    println!("   let context = EvaluationContext::builder()");
    println!("       .with_model_provider(Arc::new(registry))");
    println!("       .build();");
    println!("   ```");

    println!("\n=== Key Benefits of Runtime Loading ===");
    println!("✓ Use official HL7 ModelInfo files (not placeholders)");
    println!("✓ Update ModelInfo without recompiling");
    println!("✓ Support multiple FHIR versions simultaneously");
    println!("✓ Integrate with FHIR package manager");

    println!("\n=== Typical File Locations ===");
    println!("FHIR R4: .../hl7.fhir.r4.core#4.0.1/package/FHIR-ModelInfo-4.0.1.xml");
    println!("FHIR R5: .../hl7.fhir.r5.core#5.0.0/package/FHIR-ModelInfo-5.0.0.xml");
    println!("\nSee: http://cql.hl7.org/09-b-cqlreference.html#fhir-4-0-1-modelinfo");

    Ok(())
}
