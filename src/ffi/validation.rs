//! FFI validation rules

use super::errors::FFIError;
use std::sync::Arc;

/// Validation rule for FFI configuration
/// Uses an opaque context type to avoid circular dependencies
pub struct ValidationRule {
    pub name: String,
    pub validator: Arc<dyn Fn(&ValidationContext) -> Result<(), FFIError> + Send + Sync>,
}

/// Context for validation - provides access to builder fields without direct dependency
pub struct ValidationContext<'a> {
    pub name: &'a str,
    pub search_paths: &'a [std::path::PathBuf],
}

impl ValidationRule {
    /// Create a new validation rule
    pub fn new<F>(name: impl Into<String>, validator: F) -> Self
    where
        F: Fn(&ValidationContext) -> Result<(), FFIError> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            validator: Arc::new(validator),
        }
    }

    /// Validate with the provided context
    pub fn validate_with_context(&self, context: &ValidationContext) -> Result<(), FFIError> {
        (self.validator)(context)
    }
}

impl std::fmt::Debug for ValidationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationRule")
            .field("name", &self.name)
            .field("validator", &"<function>")
            .finish()
    }
}
