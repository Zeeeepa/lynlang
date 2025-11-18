// FFI validation rules

use super::errors::FFIError;
use super::builder::LibBuilder;
use std::sync::Arc;

/// Validation rule for FFI configuration
pub struct ValidationRule {
    pub name: String,
    pub validator: Arc<dyn Fn(&LibBuilder) -> Result<(), FFIError> + Send + Sync>,
}

impl ValidationRule {
    pub fn validate(&self, builder: &LibBuilder) -> Result<(), FFIError> {
        (self.validator)(builder)
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

