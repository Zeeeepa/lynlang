// FFI Module - Foreign Function Interface with Builder Pattern
// Implements the Language Spec v1.1.0 requirements for safe C interop

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::PathBuf;
use libloading::Library as DynLib;

use crate::ast::AstType;

/// FFI builder for safe C interop
pub struct FFI;

impl FFI {
    /// Create a new library builder
    pub fn lib(name: impl Into<String>) -> LibBuilder {
        LibBuilder::new(name.into())
    }
}

/// Builder for configuring a foreign library
pub struct LibBuilder {
    name: String,
    path: Option<PathBuf>,
    functions: HashMap<String, FnSignature>,
    constants: HashMap<String, AstType>,
}

impl LibBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            path: None,
            functions: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    /// Set the library path
    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add a function to the library
    pub fn function(mut self, name: impl Into<String>, sig: FnSignature) -> Self {
        self.functions.insert(name.into(), sig);
        self
    }

    /// Add a constant to the library
    pub fn constant(mut self, name: impl Into<String>, typ: AstType) -> Self {
        self.constants.insert(name.into(), typ);
        self
    }

    /// Build the library handle
    pub fn build(self) -> Result<Library, FFIError> {
        let path = self.path.unwrap_or_else(|| {
            Self::default_lib_path(&self.name)
        });

        Ok(Library {
            name: self.name,
            path,
            functions: self.functions,
            constants: self.constants,
            handle: None,
        })
    }

    /// Determine default library path based on platform
    fn default_lib_path(name: &str) -> PathBuf {
        #[cfg(target_os = "linux")]
        let filename = format!("lib{}.so", name);
        
        #[cfg(target_os = "macos")]
        let filename = format!("lib{}.dylib", name);
        
        #[cfg(target_os = "windows")]
        let filename = format!("{}.dll", name);
        
        PathBuf::from(filename)
    }
}

/// Function signature for FFI
#[derive(Debug, Clone, PartialEq)]
pub struct FnSignature {
    pub params: Vec<AstType>,
    pub returns: AstType,
}

impl FnSignature {
    pub fn new(params: Vec<AstType>, returns: AstType) -> Self {
        Self { params, returns }
    }
}

/// Represents a loaded dynamic library
pub struct Library {
    name: String,
    path: PathBuf,
    functions: HashMap<String, FnSignature>,
    constants: HashMap<String, AstType>,
    handle: Option<DynLib>,
}

impl Library {
    /// Load the dynamic library
    pub fn load(&mut self) -> Result<(), FFIError> {
        match unsafe { DynLib::new(&self.path) } {
            Ok(lib) => {
                self.handle = Some(lib);
                Ok(())
            }
            Err(e) => Err(FFIError::LibraryNotFound {
                path: self.path.display().to_string(),
                error: e.to_string(),
            })
        }
    }

    /// Get a function from the library
    pub fn get_function(&self, name: &str) -> Result<*mut c_void, FFIError> {
        let handle = self.handle.as_ref()
            .ok_or(FFIError::LibraryNotLoaded)?;

        // Verify function signature exists
        if !self.functions.contains_key(name) {
            return Err(FFIError::SymbolNotFound(name.to_string()));
        }

        // Load the symbol
        let symbol_name = CString::new(name)
            .map_err(|_| FFIError::InvalidSymbolName(name.to_string()))?;

        unsafe {
            match handle.get::<*mut c_void>(name.as_bytes()) {
                Ok(sym) => Ok(*sym),
                Err(e) => Err(FFIError::SymbolNotFound(name.to_string()))
            }
        }
    }

    /// Get a constant from the library
    pub fn get_constant(&self, name: &str) -> Result<*mut c_void, FFIError> {
        // Constants are just symbols
        self.get_function(name)
    }

    /// Unload the library
    pub fn unload(&mut self) {
        self.handle = None;
    }

    /// Get the function signature for a given function
    pub fn get_signature(&self, name: &str) -> Option<&FnSignature> {
        self.functions.get(name)
    }

    /// Check if library is loaded
    pub fn is_loaded(&self) -> bool {
        self.handle.is_some()
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        self.unload();
    }
}

/// FFI Error types
#[derive(Debug, Clone)]
pub enum FFIError {
    LibraryNotFound { path: String, error: String },
    SymbolNotFound(String),
    LibraryNotLoaded,
    InvalidSignature { expected: FnSignature, got: FnSignature },
    InvalidSymbolName(String),
}

impl std::fmt::Display for FFIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FFIError::LibraryNotFound { path, error } => 
                write!(f, "Library not found at '{}': {}", path, error),
            FFIError::SymbolNotFound(name) => 
                write!(f, "Symbol '{}' not found in library", name),
            FFIError::LibraryNotLoaded => 
                write!(f, "Library not loaded"),
            FFIError::InvalidSignature { expected, got } => 
                write!(f, "Invalid function signature: expected {:?}, got {:?}", expected, got),
            FFIError::InvalidSymbolName(name) => 
                write!(f, "Invalid symbol name: '{}'", name),
        }
    }
}

impl std::error::Error for FFIError {}

/// Helper module for creating FFI types
pub mod types {
    use crate::ast::AstType;

    pub fn void() -> AstType {
        AstType::Void
    }

    pub fn bool() -> AstType {
        AstType::Bool
    }

    pub fn i8() -> AstType {
        AstType::I8
    }

    pub fn i16() -> AstType {
        AstType::I16
    }

    pub fn i32() -> AstType {
        AstType::I32
    }

    pub fn i64() -> AstType {
        AstType::I64
    }

    pub fn u8() -> AstType {
        AstType::U8
    }

    pub fn u16() -> AstType {
        AstType::U16
    }

    pub fn u32() -> AstType {
        AstType::U32
    }

    pub fn u64() -> AstType {
        AstType::U64
    }

    pub fn f32() -> AstType {
        AstType::F32
    }

    pub fn f64() -> AstType {
        AstType::F64
    }

    pub fn usize() -> AstType {
        // Use U64 as platform-specific usize equivalent
        AstType::U64
    }

    pub fn string() -> AstType {
        AstType::String
    }

    pub fn raw_ptr(inner: AstType) -> AstType {
        // Use regular Pointer for raw pointers in FFI
        AstType::Pointer(Box::new(inner))
    }

    pub fn ptr(inner: AstType) -> AstType {
        // Use regular Pointer type
        AstType::Pointer(Box::new(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_builder_pattern() {
        // Test building a library configuration
        let lib = FFI::lib("sqlite3")
            .path("/usr/lib/libsqlite3.so")
            .function("sqlite3_open", FnSignature::new(
                vec![types::string(), types::raw_ptr(types::raw_ptr(types::void()))],
                types::i32(),
            ))
            .function("sqlite3_close", FnSignature::new(
                vec![types::raw_ptr(types::void())],
                types::i32(),
            ))
            .constant("SQLITE_OK", types::i32())
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(lib.name, "sqlite3");
        assert_eq!(lib.path, PathBuf::from("/usr/lib/libsqlite3.so"));
        assert_eq!(lib.functions.len(), 2);
        assert_eq!(lib.constants.len(), 1);
    }

    #[test]
    fn test_default_lib_path() {
        let path = LibBuilder::default_lib_path("test");
        
        #[cfg(target_os = "linux")]
        assert_eq!(path, PathBuf::from("libtest.so"));
        
        #[cfg(target_os = "macos")]
        assert_eq!(path, PathBuf::from("libtest.dylib"));
        
        #[cfg(target_os = "windows")]
        assert_eq!(path, PathBuf::from("test.dll"));
    }
}