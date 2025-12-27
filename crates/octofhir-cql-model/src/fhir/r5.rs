//! FHIR R5 ModelInfo
//!
//! Embedded FHIR R5 ModelInfo for CQL evaluation.

use crate::model_info::ModelInfo;
use crate::provider::ModelProviderError;
use crate::registry::ModelRegistry;
use once_cell::sync::Lazy;

/// FHIR R5 ModelInfo XML (embedded at compile time)
/// This will be populated once we download the actual ModelInfo file
pub const FHIR_R5_MODEL_INFO_XML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/FHIR-modelinfo-5.0.0.xml"));

/// Lazily initialized FHIR R5 model registry
pub static FHIR_R5_REGISTRY: Lazy<Result<ModelRegistry, ModelProviderError>> =
    Lazy::new(|| ModelRegistry::from_xml(FHIR_R5_MODEL_INFO_XML));

/// Get the FHIR R5 model registry
pub fn fhir_r5_registry() -> Result<ModelRegistry, ModelProviderError> {
    FHIR_R5_REGISTRY.clone()
}

/// Load FHIR R5 ModelInfo from embedded resource
pub fn load_fhir_r5_model_info() -> Result<ModelInfo, ModelProviderError> {
    crate::model_info::parse_xml(FHIR_R5_MODEL_INFO_XML)
        .map_err(|e| ModelProviderError::ParseError(e.to_string()))
}
