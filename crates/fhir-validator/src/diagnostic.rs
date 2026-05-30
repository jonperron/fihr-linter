use std::sync::Arc;

use fhir_parser::Span;

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A validation finding emitted by the validator or linter.
///
/// `Diagnostic` is the single output type across all crates.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Machine-readable code, e.g. `"CARDINALITY_001"`.
    pub code: Arc<str>,
    pub message: String,
    /// FHIRPath to the offending node, e.g. `"Patient.name[0].given"`.
    pub path: String,
    /// Source location, when available.
    pub location: Option<Span>,
    /// Lint rule identifier, if this diagnostic originated from a lint rule.
    pub rule: Option<Arc<str>>,
}

impl Diagnostic {
    pub fn error(code: &str, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: Arc::from(code),
            message: message.into(),
            path: path.into(),
            location: None,
            rule: None,
        }
    }

    pub fn warning(code: &str, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: Arc::from(code),
            message: message.into(),
            path: path.into(),
            location: None,
            rule: None,
        }
    }

    pub fn with_location(mut self, location: Span) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_rule(mut self, rule: impl Into<Arc<str>>) -> Self {
        self.rule = Some(rule.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_sets_severity_and_code() {
        let d = Diagnostic::error("TEST_001", "test message", "Patient.foo");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.code.as_ref(), "TEST_001");
        assert_eq!(d.message, "test message");
        assert_eq!(d.path, "Patient.foo");
        assert!(d.location.is_none());
        assert!(d.rule.is_none());
    }

    #[test]
    fn warning_sets_severity() {
        let d = Diagnostic::warning("TEST_002", "warn", "Patient.bar");
        assert_eq!(d.severity, Severity::Warning);
    }

    #[test]
    fn with_location_attaches_span() {
        let span = Span {
            line: 3,
            col: 5,
            offset: 42,
        };
        let d = Diagnostic::error("X", "msg", "p").with_location(span);
        assert_eq!(d.location, Some(span));
    }

    #[test]
    fn with_rule_attaches_rule_id() {
        let d = Diagnostic::warning("X", "msg", "p").with_rule("my-rule");
        assert_eq!(d.rule.as_deref(), Some("my-rule"));
    }

    #[test]
    fn severity_ordering_error_before_warning() {
        assert!(Severity::Error < Severity::Warning);
        assert!(Severity::Warning < Severity::Information);
        assert!(Severity::Information < Severity::Hint);
    }
}
