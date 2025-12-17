// FFI error types

/// FFI-specific errors
#[derive(Debug, Clone)]
pub enum FFIError {
    LibraryNotFound { path: String, error: String },
    SymbolNotFound(String),
    InvalidSignature { function: String, reason: String },
    InvalidSymbolName(String),
    ValidationError(String),
    LibraryNotLoaded,
    CallFailed { function: String, error: String },
}

impl std::fmt::Display for FFIError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FFIError::LibraryNotFound { path, error } => {
                write!(f, "Library not found at '{}': {}", path, error)
            }
            FFIError::SymbolNotFound(name) => {
                write!(f, "Symbol '{}' not found in library", name)
            }
            FFIError::InvalidSignature { function, reason } => {
                write!(
                    f,
                    "Invalid signature for function '{}': {}",
                    function, reason
                )
            }
            FFIError::InvalidSymbolName(name) => {
                write!(f, "Invalid symbol name: {}", name)
            }
            FFIError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
            FFIError::LibraryNotLoaded => {
                write!(f, "Library is not loaded")
            }
            FFIError::CallFailed { function, error } => {
                write!(f, "Call to function '{}' failed: {}", function, error)
            }
        }
    }
}

impl std::error::Error for FFIError {}
