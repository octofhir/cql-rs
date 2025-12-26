//! CQL error codes following a structured numbering system
//!
//! Error code ranges:
//! - CQL0001-CQL0099: Parse errors (syntax)
//! - CQL0100-CQL0199: Semantic errors (type checking, resolution)
//! - CQL0200-CQL0299: Evaluation errors (runtime)
//! - CQL0300-CQL0399: Model errors (FHIR, data model)
//! - CQL0400-CQL0499: System errors (I/O, configuration)

use serde::{Deserialize, Serialize};
use std::fmt;

/// Error code identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErrorCode(u16);

impl ErrorCode {
    /// Create a new error code
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    /// Get the numeric code
    pub const fn code(&self) -> u16 {
        self.0
    }

    /// Get error information for this code
    pub fn info(&self) -> &'static ErrorInfo {
        ERROR_INFO.get(&self.0).unwrap_or(&UNKNOWN_ERROR)
    }

    /// Check if this is a parse error (0001-0099)
    pub const fn is_parse_error(&self) -> bool {
        self.0 >= 1 && self.0 < 100
    }

    /// Check if this is a semantic error (0100-0199)
    pub const fn is_semantic_error(&self) -> bool {
        self.0 >= 100 && self.0 < 200
    }

    /// Check if this is an evaluation error (0200-0299)
    pub const fn is_evaluation_error(&self) -> bool {
        self.0 >= 200 && self.0 < 300
    }

    /// Check if this is a model error (0300-0399)
    pub const fn is_model_error(&self) -> bool {
        self.0 >= 300 && self.0 < 400
    }

    /// Check if this is a system error (0400-0499)
    pub const fn is_system_error(&self) -> bool {
        self.0 >= 400 && self.0 < 500
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CQL{:04}", self.0)
    }
}

/// Information about an error code
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// Short description of the error
    pub description: &'static str,
    /// Detailed help text
    pub help: Option<&'static str>,
    /// Link to documentation
    pub docs_url: Option<&'static str>,
}

impl ErrorInfo {
    const fn new(description: &'static str) -> Self {
        Self {
            description,
            help: None,
            docs_url: None,
        }
    }

    const fn with_help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }
}

// Static error info storage
static UNKNOWN_ERROR: ErrorInfo = ErrorInfo::new("Unknown error");

use std::collections::HashMap;
use std::sync::LazyLock;

static ERROR_INFO: LazyLock<HashMap<u16, ErrorInfo>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    // Parse errors (0001-0099)
    map.insert(1, ErrorInfo::new("Unexpected token"));
    map.insert(2, ErrorInfo::new("Unexpected end of input"));
    map.insert(3, ErrorInfo::new("Invalid identifier"));
    map.insert(4, ErrorInfo::new("Invalid literal"));
    map.insert(5, ErrorInfo::new("Invalid string escape sequence"));
    map.insert(6, ErrorInfo::new("Unterminated string literal"));
    map.insert(7, ErrorInfo::new("Invalid number format"));
    map.insert(8, ErrorInfo::new("Invalid date/time format"));
    map.insert(9, ErrorInfo::new("Invalid quantity format"));
    map.insert(10, ErrorInfo::new("Missing closing delimiter"));
    map.insert(11, ErrorInfo::new("Missing opening delimiter"));
    map.insert(12, ErrorInfo::new("Expected expression"));
    map.insert(13, ErrorInfo::new("Expected identifier"));
    map.insert(14, ErrorInfo::new("Expected type specifier"));
    map.insert(15, ErrorInfo::new("Invalid operator"));
    map.insert(16, ErrorInfo::new("Invalid keyword usage"));
    map.insert(17, ErrorInfo::new("Invalid library definition"));
    map.insert(18, ErrorInfo::new("Invalid using definition"));
    map.insert(19, ErrorInfo::new("Invalid include definition"));
    map.insert(20, ErrorInfo::new("Invalid parameter definition"));
    map.insert(21, ErrorInfo::new("Invalid codesystem definition"));
    map.insert(22, ErrorInfo::new("Invalid valueset definition"));
    map.insert(23, ErrorInfo::new("Invalid code definition"));
    map.insert(24, ErrorInfo::new("Invalid concept definition"));
    map.insert(25, ErrorInfo::new("Invalid context definition"));
    map.insert(26, ErrorInfo::new("Invalid function definition"));
    map.insert(27, ErrorInfo::new("Invalid query expression"));
    map.insert(28, ErrorInfo::new("Invalid retrieve expression"));
    map.insert(29, ErrorInfo::new("Invalid interval expression"));
    map.insert(30, ErrorInfo::new("Invalid list expression"));
    map.insert(31, ErrorInfo::new("Invalid tuple expression"));
    map.insert(32, ErrorInfo::new("Invalid case expression"));
    map.insert(33, ErrorInfo::new("Invalid if expression"));
    map.insert(34, ErrorInfo::new("Invalid aggregate expression"));
    map.insert(35, ErrorInfo::new("Reserved keyword"));

    // Semantic errors (0100-0199)
    map.insert(100, ErrorInfo::new("Undefined identifier")
        .with_help("Check that the identifier is defined in scope"));
    map.insert(101, ErrorInfo::new("Undefined function"));
    map.insert(102, ErrorInfo::new("Undefined type"));
    map.insert(103, ErrorInfo::new("Undefined library"));
    map.insert(104, ErrorInfo::new("Undefined codesystem"));
    map.insert(105, ErrorInfo::new("Undefined valueset"));
    map.insert(106, ErrorInfo::new("Undefined code"));
    map.insert(107, ErrorInfo::new("Undefined concept"));
    map.insert(108, ErrorInfo::new("Undefined parameter"));
    map.insert(109, ErrorInfo::new("Duplicate definition"));
    map.insert(110, ErrorInfo::new("Type mismatch"));
    map.insert(111, ErrorInfo::new("Invalid argument count"));
    map.insert(112, ErrorInfo::new("Invalid argument type"));
    map.insert(113, ErrorInfo::new("Ambiguous function call"));
    map.insert(114, ErrorInfo::new("Circular reference"));
    map.insert(115, ErrorInfo::new("Invalid cast"));
    map.insert(116, ErrorInfo::new("Invalid comparison"));
    map.insert(117, ErrorInfo::new("Invalid operation"));
    map.insert(118, ErrorInfo::new("Context not established"));
    map.insert(119, ErrorInfo::new("Invalid retrieve"));
    map.insert(120, ErrorInfo::new("Invalid property access"));

    // Evaluation errors (0200-0299)
    map.insert(200, ErrorInfo::new("Evaluation failed"));
    map.insert(201, ErrorInfo::new("Null value error"));
    map.insert(202, ErrorInfo::new("Division by zero"));
    map.insert(203, ErrorInfo::new("Overflow error"));
    map.insert(204, ErrorInfo::new("Underflow error"));
    map.insert(205, ErrorInfo::new("Invalid conversion"));
    map.insert(206, ErrorInfo::new("Invalid index"));
    map.insert(207, ErrorInfo::new("Invalid slice"));
    map.insert(208, ErrorInfo::new("Invalid interval operation"));
    map.insert(209, ErrorInfo::new("Invalid list operation"));
    map.insert(210, ErrorInfo::new("Invalid date/time operation"));
    map.insert(211, ErrorInfo::new("Invalid quantity operation"));
    map.insert(212, ErrorInfo::new("Invalid string operation"));
    map.insert(213, ErrorInfo::new("Retrieve failed"));
    map.insert(214, ErrorInfo::new("External function failed"));
    map.insert(215, ErrorInfo::new("Timeout"));
    map.insert(216, ErrorInfo::new("Resource limit exceeded"));

    // Model errors (0300-0399)
    map.insert(300, ErrorInfo::new("Model not found"));
    map.insert(301, ErrorInfo::new("Invalid model version"));
    map.insert(302, ErrorInfo::new("Type not found in model"));
    map.insert(303, ErrorInfo::new("Property not found"));
    map.insert(304, ErrorInfo::new("Invalid profile"));
    map.insert(305, ErrorInfo::new("ModelInfo load failed"));
    map.insert(306, ErrorInfo::new("Terminology lookup failed"));
    map.insert(307, ErrorInfo::new("Code validation failed"));
    map.insert(308, ErrorInfo::new("ValueSet expansion failed"));

    // System errors (0400-0499)
    map.insert(400, ErrorInfo::new("Internal error"));
    map.insert(401, ErrorInfo::new("I/O error"));
    map.insert(402, ErrorInfo::new("Configuration error"));
    map.insert(403, ErrorInfo::new("Network error"));
    map.insert(404, ErrorInfo::new("File not found"));
    map.insert(405, ErrorInfo::new("Permission denied"));
    map.insert(406, ErrorInfo::new("Invalid format"));

    map
});

// Convenient error code constants

// Parse errors
pub const CQL0001: ErrorCode = ErrorCode::new(1);
pub const CQL0002: ErrorCode = ErrorCode::new(2);
pub const CQL0003: ErrorCode = ErrorCode::new(3);
pub const CQL0004: ErrorCode = ErrorCode::new(4);
pub const CQL0005: ErrorCode = ErrorCode::new(5);
pub const CQL0006: ErrorCode = ErrorCode::new(6);
pub const CQL0007: ErrorCode = ErrorCode::new(7);
pub const CQL0008: ErrorCode = ErrorCode::new(8);
pub const CQL0009: ErrorCode = ErrorCode::new(9);
pub const CQL0010: ErrorCode = ErrorCode::new(10);
pub const CQL0011: ErrorCode = ErrorCode::new(11);
pub const CQL0012: ErrorCode = ErrorCode::new(12);
pub const CQL0013: ErrorCode = ErrorCode::new(13);
pub const CQL0014: ErrorCode = ErrorCode::new(14);
pub const CQL0015: ErrorCode = ErrorCode::new(15);
pub const CQL0016: ErrorCode = ErrorCode::new(16);
pub const CQL0017: ErrorCode = ErrorCode::new(17);
pub const CQL0018: ErrorCode = ErrorCode::new(18);
pub const CQL0019: ErrorCode = ErrorCode::new(19);
pub const CQL0020: ErrorCode = ErrorCode::new(20);
pub const CQL0021: ErrorCode = ErrorCode::new(21);
pub const CQL0022: ErrorCode = ErrorCode::new(22);
pub const CQL0023: ErrorCode = ErrorCode::new(23);
pub const CQL0024: ErrorCode = ErrorCode::new(24);
pub const CQL0025: ErrorCode = ErrorCode::new(25);
pub const CQL0026: ErrorCode = ErrorCode::new(26);
pub const CQL0027: ErrorCode = ErrorCode::new(27);
pub const CQL0028: ErrorCode = ErrorCode::new(28);
pub const CQL0029: ErrorCode = ErrorCode::new(29);
pub const CQL0030: ErrorCode = ErrorCode::new(30);
pub const CQL0031: ErrorCode = ErrorCode::new(31);
pub const CQL0032: ErrorCode = ErrorCode::new(32);
pub const CQL0033: ErrorCode = ErrorCode::new(33);
pub const CQL0034: ErrorCode = ErrorCode::new(34);
pub const CQL0035: ErrorCode = ErrorCode::new(35);

// Semantic errors
pub const CQL0100: ErrorCode = ErrorCode::new(100);
pub const CQL0101: ErrorCode = ErrorCode::new(101);
pub const CQL0102: ErrorCode = ErrorCode::new(102);
pub const CQL0103: ErrorCode = ErrorCode::new(103);
pub const CQL0104: ErrorCode = ErrorCode::new(104);
pub const CQL0105: ErrorCode = ErrorCode::new(105);
pub const CQL0106: ErrorCode = ErrorCode::new(106);
pub const CQL0107: ErrorCode = ErrorCode::new(107);
pub const CQL0108: ErrorCode = ErrorCode::new(108);
pub const CQL0109: ErrorCode = ErrorCode::new(109);
pub const CQL0110: ErrorCode = ErrorCode::new(110);
pub const CQL0111: ErrorCode = ErrorCode::new(111);
pub const CQL0112: ErrorCode = ErrorCode::new(112);
pub const CQL0113: ErrorCode = ErrorCode::new(113);
pub const CQL0114: ErrorCode = ErrorCode::new(114);
pub const CQL0115: ErrorCode = ErrorCode::new(115);
pub const CQL0116: ErrorCode = ErrorCode::new(116);
pub const CQL0117: ErrorCode = ErrorCode::new(117);
pub const CQL0118: ErrorCode = ErrorCode::new(118);
pub const CQL0119: ErrorCode = ErrorCode::new(119);
pub const CQL0120: ErrorCode = ErrorCode::new(120);

// Evaluation errors
pub const CQL0200: ErrorCode = ErrorCode::new(200);
pub const CQL0201: ErrorCode = ErrorCode::new(201);
pub const CQL0202: ErrorCode = ErrorCode::new(202);
pub const CQL0203: ErrorCode = ErrorCode::new(203);
pub const CQL0204: ErrorCode = ErrorCode::new(204);
pub const CQL0205: ErrorCode = ErrorCode::new(205);
pub const CQL0206: ErrorCode = ErrorCode::new(206);
pub const CQL0207: ErrorCode = ErrorCode::new(207);
pub const CQL0208: ErrorCode = ErrorCode::new(208);
pub const CQL0209: ErrorCode = ErrorCode::new(209);
pub const CQL0210: ErrorCode = ErrorCode::new(210);
pub const CQL0211: ErrorCode = ErrorCode::new(211);
pub const CQL0212: ErrorCode = ErrorCode::new(212);
pub const CQL0213: ErrorCode = ErrorCode::new(213);
pub const CQL0214: ErrorCode = ErrorCode::new(214);
pub const CQL0215: ErrorCode = ErrorCode::new(215);
pub const CQL0216: ErrorCode = ErrorCode::new(216);

// Model errors
pub const CQL0300: ErrorCode = ErrorCode::new(300);
pub const CQL0301: ErrorCode = ErrorCode::new(301);
pub const CQL0302: ErrorCode = ErrorCode::new(302);
pub const CQL0303: ErrorCode = ErrorCode::new(303);
pub const CQL0304: ErrorCode = ErrorCode::new(304);
pub const CQL0305: ErrorCode = ErrorCode::new(305);
pub const CQL0306: ErrorCode = ErrorCode::new(306);
pub const CQL0307: ErrorCode = ErrorCode::new(307);
pub const CQL0308: ErrorCode = ErrorCode::new(308);

// System errors
pub const CQL0400: ErrorCode = ErrorCode::new(400);
pub const CQL0401: ErrorCode = ErrorCode::new(401);
pub const CQL0402: ErrorCode = ErrorCode::new(402);
pub const CQL0403: ErrorCode = ErrorCode::new(403);
pub const CQL0404: ErrorCode = ErrorCode::new(404);
pub const CQL0405: ErrorCode = ErrorCode::new(405);
pub const CQL0406: ErrorCode = ErrorCode::new(406);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(CQL0001.to_string(), "CQL0001");
        assert_eq!(CQL0100.to_string(), "CQL0100");
    }

    #[test]
    fn test_error_categories() {
        assert!(CQL0001.is_parse_error());
        assert!(!CQL0001.is_semantic_error());

        assert!(CQL0100.is_semantic_error());
        assert!(!CQL0100.is_parse_error());

        assert!(CQL0200.is_evaluation_error());
        assert!(CQL0300.is_model_error());
        assert!(CQL0400.is_system_error());
    }

    #[test]
    fn test_error_info() {
        let info = CQL0001.info();
        assert_eq!(info.description, "Unexpected token");
    }
}
