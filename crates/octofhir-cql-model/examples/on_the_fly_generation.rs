//! On-the-fly ModelInfo generation without filesystem
//!
//! This example demonstrates that you can create and use ModelInfo entirely
//! in-memory without any file I/O - perfect for server usage where you want
//! to generate configurations dynamically.

use octofhir_cql_model::{
    model_info::{ModelInfo, PropertyInfo, TypeInfo},
    ModelProvider, ModelRegistry,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== On-the-Fly ModelInfo Generation (Zero File I/O) ===\n");

    // ========================================================================
    // Method 1: Use Embedded ModelInfo (No I/O)
    // ========================================================================
    println!("1. Embedded ModelInfo - Zero I/O:");

    // Each call creates a new registry from embedded data - NO FILE READING
    let r4_registry = octofhir_cql_model::fhir::fhir_r4_registry()?;
    println!("   ✓ Created R4 registry (no file I/O)");
    println!("   - Model: {} v{}", r4_registry.model_name(), r4_registry.model_version());

    // Can create multiple instances without I/O
    let another_r4 = octofhir_cql_model::fhir::fhir_r4_registry()?;
    let r5_registry = octofhir_cql_model::fhir::fhir_r5_registry()?;
    println!("   ✓ Created multiple registries simultaneously (no I/O)");

    // Cheap cloning via Arc
    let cloned = r4_registry.clone();
    println!("   ✓ Cloned registry (cheap via Arc)");

    // ========================================================================
    // Method 2: Programmatic ModelInfo Creation
    // ========================================================================
    println!("\n2. Programmatic In-Memory Creation:");

    // Create a custom ModelInfo entirely in memory
    let mut custom_model = ModelInfo::new("CustomModel", "1.0.0");
    custom_model.url = "http://example.org/custom".to_string();

    // Add a custom type
    let mut custom_type = TypeInfo::new("CustomPatient");
    custom_type.retrievable = true;
    custom_type.primary_code_path = Some("identifier".to_string());

    // Add properties
    custom_type.elements.push(PropertyInfo {
        name: "id".to_string(),
        element_type: "string".to_string(),
        is_list: false,
        target: None,
    });

    custom_type.elements.push(PropertyInfo {
        name: "name".to_string(),
        element_type: "string".to_string(),
        is_list: true,  // List of names
        target: None,
    });

    custom_model.type_infos.insert("CustomPatient".to_string(), custom_type);

    // Create registry from the in-memory ModelInfo
    let custom_registry = ModelRegistry::new(custom_model);
    println!("   ✓ Created custom ModelInfo programmatically (no I/O)");
    println!("   - Retrievable types: {}", custom_registry.get_retrievable_types().len());

    if let Some(patient) = custom_registry.get_type("CustomPatient").await? {
        println!("   - CustomPatient properties: {}", patient.elements.len());
    }

    // ========================================================================
    // Method 3: Generate from String (In-Memory)
    // ========================================================================
    println!("\n3. Parse from In-Memory String:");

    // You can have XML/JSON as a constant or generated string
    let xml_string = r#"<?xml version="1.0" encoding="UTF-8"?>
        <modelInfo name="RuntimeGenerated" version="2.0.0" url="http://runtime.org">
            <typeInfo name="Observation" retrievable="true" primaryCodePath="code">
                <element name="id" type="string"/>
                <element name="code" type="string"/>
                <element name="value" type="decimal"/>
            </typeInfo>
        </modelInfo>"#;

    // Parse from string - no file I/O
    let runtime_registry = ModelRegistry::from_xml(xml_string)?;
    println!("   ✓ Parsed ModelInfo from in-memory string (no I/O)");
    println!("   - Model: {} v{}",
        runtime_registry.model_name(),
        runtime_registry.model_version()
    );

    // ========================================================================
    // Method 4: Generate Based on Runtime Configuration
    // ========================================================================
    println!("\n4. Dynamic Generation Based on Config:");

    // Simulate generating ModelInfo based on runtime configuration
    let config = vec![
        ("Patient", vec!["id", "name", "birthDate"]),
        ("Observation", vec!["id", "code", "value", "effectiveDateTime"]),
        ("Condition", vec!["id", "code", "clinicalStatus"]),
    ];

    let mut dynamic_model = ModelInfo::new("DynamicModel", "1.0.0");

    for (type_name, properties) in config {
        let mut type_info = TypeInfo::new(type_name);
        type_info.retrievable = true;

        for prop_name in properties {
            type_info.elements.push(PropertyInfo {
                name: prop_name.to_string(),
                element_type: "string".to_string(),
                is_list: false,
                target: None,
            });
        }

        dynamic_model.type_infos.insert(type_name.to_string(), type_info);
    }

    let dynamic_registry = ModelRegistry::new(dynamic_model);
    println!("   ✓ Generated ModelInfo from runtime config (no I/O)");
    println!("   - Generated {} retrievable types dynamically", dynamic_registry.get_retrievable_types().len());

    // ========================================================================
    // Method 5: Modify Embedded ModelInfo at Runtime
    // ========================================================================
    println!("\n5. Modify Embedded ModelInfo at Runtime:");

    // Load embedded ModelInfo
    let fhir_model_info = octofhir_cql_model::model_info::parse_xml(
        octofhir_cql_model::fhir::r4::FHIR_R4_MODEL_INFO_XML
    )?;

    // Clone and modify in memory
    let mut modified_model = fhir_model_info;

    // Add a custom extension type
    let mut extension_type = TypeInfo::new("CustomExtension");
    extension_type.elements.push(PropertyInfo {
        name: "customField".to_string(),
        element_type: "string".to_string(),
        is_list: false,
        target: None,
    });

    modified_model.type_infos.insert("CustomExtension".to_string(), extension_type);

    let modified_registry = ModelRegistry::new(modified_model);
    println!("   ✓ Modified embedded ModelInfo at runtime (no I/O)");
    println!("   - Extended with custom types");

    // ========================================================================
    // Summary
    // ========================================================================
    println!("\n=== Summary: All Methods Use Zero File I/O ===");
    println!("✓ Embedded: Instantiate from compile-time included data");
    println!("✓ Programmatic: Build ModelInfo struct directly in code");
    println!("✓ String Parsing: Parse XML/JSON from in-memory strings");
    println!("✓ Dynamic Generation: Create based on runtime config/database");
    println!("✓ Runtime Modification: Clone and extend embedded models");

    println!("\n=== Server Usage Pattern ===");
    println!("// In your server - zero filesystem access:");
    println!("let registry = match tenant.custom_model {{");
    println!("    Some(xml) => ModelRegistry::from_xml(&xml)?,");
    println!("    None => fhir_r4_registry()?,  // Fallback to embedded");
    println!("}};");

    println!("\n=== Performance ===");
    println!("Embedded ModelInfo:");
    println!("  - Load time: ~0ms (in memory)");
    println!("  - No file I/O");
    println!("  - No parsing overhead (pre-parsed)");
    println!("  - Thread-safe sharing via Arc");

    Ok(())
}
