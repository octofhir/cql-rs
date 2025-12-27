//! FHIR R4 ModelInfo
//!
//! Embedded FHIR R4 ModelInfo for CQL evaluation.

use crate::model_info::ModelInfo;
use crate::provider::ModelProviderError;
use crate::registry::ModelRegistry;
use once_cell::sync::Lazy;

/// FHIR R4 ModelInfo XML (embedded at compile time)
/// This will be populated once we download the actual ModelInfo file
pub const FHIR_R4_MODEL_INFO_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FHIR-modelinfo-4.0.1.xml"));

/// Lazily initialized FHIR R4 model registry
pub static FHIR_R4_REGISTRY: Lazy<Result<ModelRegistry, ModelProviderError>> =
    Lazy::new(|| ModelRegistry::from_xml(FHIR_R4_MODEL_INFO_XML));

/// Get the FHIR R4 model registry
pub fn fhir_r4_registry() -> Result<ModelRegistry, ModelProviderError> {
    FHIR_R4_REGISTRY.clone()
}

/// Load FHIR R4 ModelInfo from embedded resource
pub fn load_fhir_r4_model_info() -> Result<ModelInfo, ModelProviderError> {
    crate::model_info::parse_xml(FHIR_R4_MODEL_INFO_XML)
        .map_err(|e| ModelProviderError::ParseError(e.to_string()))
}
