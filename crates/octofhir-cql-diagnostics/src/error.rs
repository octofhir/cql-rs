//! CQL error types

use crate::{ErrorCode, SourceLocation, Span};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Diagnostic severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Error - compilation or evaluation cannot proceed
    Error,
    /// Warning - potential issue but can continue
    Warning,
    /// Information - informational message
    Info,
    /// Hint - suggestion for improvement
    Hint,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
            Severity::Hint => write!(f, "hint"),
        }
    }
}

/// A diagnostic message with location and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity level
    pub severity: Severity,
    /// Error code
    pub code: ErrorCode,
    /// Human-readable message
    pub message: String,
    /// Source location
    pub location: Option<SourceLocation>,
    /// Additional context or help
    pub help: Option<String>,
    /// Related information
    pub related: Vec<RelatedInfo>,
}

impl Diagnostic {
    /// Create a new error diagnostic
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            location: None,
            help: None,
            related: Vec::new(),
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            location: None,
            help: None,
            related: Vec::new(),
        }
    }

    /// Set the location
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Set the span (converts to location using provided source)
    pub fn with_span(mut self, span: Span, source: &str) -> Self {
        self.location = Some(SourceLocation::from_span(span, source));
        self
    }

    /// Set help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add related information
    pub fn with_related(mut self, info: RelatedInfo) -> Self {
        self.related.push(info);
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} - {}", self.severity, self.code, self.message)?;
        if let Some(loc) = &self.location {
            write!(f, " at {}", loc)?;
        }
        Ok(())
    }
}

/// Related diagnostic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedInfo {
    /// Location of related code
    pub location: Option<SourceLocation>,
    /// Message explaining the relationship
    pub message: String,
}

impl RelatedInfo {
    /// Create new related info
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            location: None,
            message: message.into(),
        }
    }

    /// Set the location
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }
}

/// Main CQL error type
#[derive(Debug, Clone, Error)]
pub enum CqlError {
    /// Parse error
    #[error("{code}: {message}")]
    Parse {
        code: ErrorCode,
        message: String,
        expression: String,
        location: Option<SourceLocation>,
        context: Option<String>,
    },

    /// Semantic error (type checking, resolution)
    #[error("{code}: {message}")]
    Semantic {
        code: ErrorCode,
        message: String,
        location: Option<SourceLocation>,
        context: Option<String>,
    },

    /// Evaluation error
    #[error("{code}: {message}")]
    Evaluation {
        code: ErrorCode,
        message: String,
        location: Option<SourceLocation>,
        context: Option<String>,
    },

    /// Model error
    #[error("{code}: {message}")]
    Model {
        code: ErrorCode,
        message: String,
        resource_type: Option<String>,
        context: Option<String>,
    },

    /// System error
    #[error("{code}: {message}")]
    System {
        code: ErrorCode,
        message: String,
        context: Option<String>,
    },

    /// Multiple errors collected
    #[error("Multiple errors: {}", .0.len())]
    Multiple(Vec<CqlError>),
}

impl CqlError {
    /// Create a parse error
    pub fn parse(
        code: ErrorCode,
        message: impl Into<String>,
        expression: impl Into<String>,
    ) -> Self {
        Self::Parse {
            code,
            message: message.into(),
            expression: expression.into(),
            location: None,
            context: None,
        }
    }

    /// Create a parse error with location
    pub fn parse_at(
        code: ErrorCode,
        message: impl Into<String>,
        expression: impl Into<String>,
        location: SourceLocation,
    ) -> Self {
        Self::Parse {
            code,
            message: message.into(),
            expression: expression.into(),
            location: Some(location),
            context: None,
        }
    }

    /// Create a semantic error
    pub fn semantic(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Semantic {
            code,
            message: message.into(),
            location: None,
            context: None,
        }
    }

    /// Create an evaluation error
    pub fn evaluation(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Evaluation {
            code,
            message: message.into(),
            location: None,
            context: None,
        }
    }

    /// Create a model error
    pub fn model(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Model {
            code,
            message: message.into(),
            resource_type: None,
            context: None,
        }
    }

    /// Create a system error
    pub fn system(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::System {
            code,
            message: message.into(),
            context: None,
        }
    }

    /// Get the error code
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Parse { code, .. } => *code,
            Self::Semantic { code, .. } => *code,
            Self::Evaluation { code, .. } => *code,
            Self::Model { code, .. } => *code,
            Self::System { code, .. } => *code,
            Self::Multiple(errors) => errors.first().map(|e| e.code()).unwrap_or(ErrorCode::new(0)),
        }
    }

    /// Get the location if available
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            Self::Parse { location, .. } => location.as_ref(),
            Self::Semantic { location, .. } => location.as_ref(),
            Self::Evaluation { location, .. } => location.as_ref(),
            _ => None,
        }
    }

    /// Convert to a diagnostic
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::Parse { code, message, location, .. } => {
                let mut diag = Diagnostic::error(*code, message.clone());
                if let Some(loc) = location {
                    diag = diag.with_location(loc.clone());
                }
                diag
            }
            Self::Semantic { code, message, location, context } => {
                let mut diag = Diagnostic::error(*code, message.clone());
                if let Some(loc) = location {
                    diag = diag.with_location(loc.clone());
                }
                if let Some(ctx) = context {
                    diag = diag.with_help(ctx.clone());
                }
                diag
            }
            Self::Evaluation { code, message, location, context } => {
                let mut diag = Diagnostic::error(*code, message.clone());
                if let Some(loc) = location {
                    diag = diag.with_location(loc.clone());
                }
                if let Some(ctx) = context {
                    diag = diag.with_help(ctx.clone());
                }
                diag
            }
            Self::Model { code, message, context, .. } => {
                let mut diag = Diagnostic::error(*code, message.clone());
                if let Some(ctx) = context {
                    diag = diag.with_help(ctx.clone());
                }
                diag
            }
            Self::System { code, message, context } => {
                let mut diag = Diagnostic::error(*code, message.clone());
                if let Some(ctx) = context {
                    diag = diag.with_help(ctx.clone());
                }
                diag
            }
            Self::Multiple(errors) => {
                if let Some(first) = errors.first() {
                    first.to_diagnostic()
                } else {
                    Diagnostic::error(ErrorCode::new(0), "Unknown error")
                }
            }
        }
    }
}

/// Builder for creating CQL errors with fluent API
pub struct ErrorBuilder {
    code: ErrorCode,
    message: String,
    location: Option<SourceLocation>,
    context: Option<String>,
}

impl ErrorBuilder {
    /// Create a new error builder
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            location: None,
            context: None,
        }
    }

    /// Set the source location
    pub fn at(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Set the span (converts to location)
    pub fn span(mut self, span: Span, source: &str) -> Self {
        self.location = Some(SourceLocation::from_span(span, source));
        self
    }

    /// Add context information
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Build a parse error
    pub fn parse(self, expression: impl Into<String>) -> CqlError {
        CqlError::Parse {
            code: self.code,
            message: self.message,
            expression: expression.into(),
            location: self.location,
            context: self.context,
        }
    }

    /// Build a semantic error
    pub fn semantic(self) -> CqlError {
        CqlError::Semantic {
            code: self.code,
            message: self.message,
            location: self.location,
            context: self.context,
        }
    }

    /// Build an evaluation error
    pub fn evaluation(self) -> CqlError {
        CqlError::Evaluation {
            code: self.code,
            message: self.message,
            location: self.location,
            context: self.context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CQL0001;

    #[test]
    fn test_error_builder() {
        let err = ErrorBuilder::new(CQL0001, "Unexpected '}'")
            .at(SourceLocation::new(1, 10, 9, 1))
            .context("Expected expression")
            .parse("define X: }");

        assert!(matches!(err, CqlError::Parse { .. }));
        assert_eq!(err.code(), CQL0001);
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::error(CQL0001, "Unexpected token")
            .with_location(SourceLocation::new(1, 5, 4, 1));

        assert!(diag.to_string().contains("CQL0001"));
        assert!(diag.to_string().contains("1:5"));
    }
}
