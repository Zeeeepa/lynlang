// FFI Module - Foreign Function Interface with Builder Pattern
// Implements the Language Spec v1.1.0 requirements for safe C interop

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
    type_mappings: HashMap<String, TypeMapping>,
    calling_convention: CallingConvention,
    safety_checks: bool,
}

impl LibBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            path: None,
            functions: HashMap::new(),
            constants: HashMap::new(),
            type_mappings: HashMap::new(),
            calling_convention: CallingConvention::C,
            safety_checks: true,
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

    /// Add a type mapping for struct/union types
    pub fn type_mapping(mut self, zen_type: impl Into<String>, mapping: TypeMapping) -> Self {
        self.type_mappings.insert(zen_type.into(), mapping);
        self
    }

    /// Set the calling convention
    pub fn calling_convention(mut self, conv: CallingConvention) -> Self {
        self.calling_convention = conv;
        self
    }

    /// Enable or disable safety checks
    pub fn safety_checks(mut self, enabled: bool) -> Self {
        self.safety_checks = enabled;
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
            type_mappings: self.type_mappings,
            calling_convention: self.calling_convention,
            safety_checks: self.safety_checks,
            handle: None,
            call_stats: Arc::new(Mutex::new(CallStatistics::new())),
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
    pub variadic: bool,
    pub safety: FunctionSafety,
}

/// Function safety level
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionSafety {
    Safe,
    Unsafe,
    Trusted,
}

impl FnSignature {
    pub fn new(params: Vec<AstType>, returns: AstType) -> Self {
        Self { 
            params, 
            returns,
            variadic: false,
            safety: FunctionSafety::Unsafe,
        }
    }

    pub fn with_variadic(mut self, variadic: bool) -> Self {
        self.variadic = variadic;
        self
    }

    pub fn with_safety(mut self, safety: FunctionSafety) -> Self {
        self.safety = safety;
        self
    }
}

/// Represents a loaded dynamic library
pub struct Library {
    name: String,
    path: PathBuf,
    functions: HashMap<String, FnSignature>,
    constants: HashMap<String, AstType>,
    type_mappings: HashMap<String, TypeMapping>,
    calling_convention: CallingConvention,
    safety_checks: bool,
    handle: Option<DynLib>,
    call_stats: Arc<Mutex<CallStatistics>>,
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

    /// Get a function from the library with safety checks
    pub fn get_function(&self, name: &str) -> Result<*mut c_void, FFIError> {
        let handle = self.handle.as_ref()
            .ok_or(FFIError::LibraryNotLoaded)?;

        // Verify function signature exists
        let signature = self.functions.get(name)
            .ok_or_else(|| FFIError::SymbolNotFound(name.to_string()))?;

        // Perform safety checks if enabled
        if self.safety_checks {
            self.validate_function_safety(name, signature)?;
        }

        // Load the symbol
        let _symbol_name = CString::new(name)
            .map_err(|_| FFIError::InvalidSymbolName(name.to_string()))?;

        unsafe {
            match handle.get::<*mut c_void>(name.as_bytes()) {
                Ok(sym) => {
                    // Record successful function lookup
                    if let Ok(mut stats) = self.call_stats.lock() {
                        stats.record_call(name, true);
                    }
                    Ok(*sym)
                },
                Err(_e) => {
                    // Record failed function lookup
                    if let Ok(mut stats) = self.call_stats.lock() {
                        stats.record_call(name, false);
                    }
                    Err(FFIError::SymbolNotFound(name.to_string()))
                }
            }
        }
    }

    /// Validate function safety
    fn validate_function_safety(&self, _name: &str, sig: &FnSignature) -> Result<(), FFIError> {
        match sig.safety {
            FunctionSafety::Safe => Ok(()),
            FunctionSafety::Trusted => {
                // Additional validation for trusted functions
                for param_type in &sig.params {
                    if matches!(param_type, AstType::Pointer(_)) {
                        // Could add more validation here
                    }
                }
                Ok(())
            },
            FunctionSafety::Unsafe => {
                // Unsafe functions always pass validation but could log warning
                Ok(())
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

    /// Get call statistics
    pub fn get_stats(&self) -> CallStatistics {
        self.call_stats.lock().unwrap().clone()
    }

    /// Call a function with automatic marshalling
    pub unsafe fn call_function<T>(
        &self,
        name: &str,
        _args: &[*const c_void],
    ) -> Result<T, FFIError> {
        let _func_ptr = self.get_function(name)?;
        // This would need platform-specific implementation
        // For now, return an error
        Err(FFIError::InvalidSymbolName("Direct calling not yet implemented".to_string()))
    }
    
    /// Get the library name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the library path
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
    
    /// Get the functions map
    pub fn functions(&self) -> &HashMap<String, FnSignature> {
        &self.functions
    }
    
    /// Get the constants map
    pub fn constants(&self) -> &HashMap<String, AstType> {
        &self.constants
    }
    
    /// Get the type mappings
    pub fn type_mappings(&self) -> &HashMap<String, TypeMapping> {
        &self.type_mappings
    }
    
    /// Get the calling convention
    pub fn calling_convention(&self) -> CallingConvention {
        self.calling_convention
    }
    
    /// Check if safety checks are enabled
    pub fn safety_checks(&self) -> bool {
        self.safety_checks
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

/// Calling convention for FFI functions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CallingConvention {
    C,
    System,
    Stdcall,
    Fastcall,
    Vectorcall,
}

/// Type mapping for complex types
#[derive(Debug, Clone)]
pub struct TypeMapping {
    pub c_type: String,
    pub zen_type: AstType,
    pub marshaller: Option<TypeMarshaller>,
}

/// Type marshalling functions
#[derive(Clone)]
pub struct TypeMarshaller {
    pub to_c: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>,
    pub from_c: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>,
}

impl std::fmt::Debug for TypeMarshaller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeMarshaller")
            .field("to_c", &"<function>")
            .field("from_c", &"<function>")
            .finish()
    }
}

/// Statistics for FFI calls
#[derive(Debug, Default, Clone)]
pub struct CallStatistics {
    pub total_calls: usize,
    pub successful_calls: usize,
    pub failed_calls: usize,
    pub function_calls: HashMap<String, usize>,
}

impl CallStatistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_call(&mut self, function: &str, success: bool) {
        self.total_calls += 1;
        if success {
            self.successful_calls += 1;
        } else {
            self.failed_calls += 1;
        }
        *self.function_calls.entry(function.to_string()).or_insert(0) += 1;
    }
}

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
            ).with_safety(FunctionSafety::Unsafe))
            .function("sqlite3_close", FnSignature::new(
                vec![types::raw_ptr(types::void())],
                types::i32(),
            ).with_safety(FunctionSafety::Unsafe))
            .constant("SQLITE_OK", types::i32())
            .calling_convention(CallingConvention::C)
            .safety_checks(true)
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(lib.name, "sqlite3");
        assert_eq!(lib.path, PathBuf::from("/usr/lib/libsqlite3.so"));
        assert_eq!(lib.functions.len(), 2);
        assert_eq!(lib.constants.len(), 1);
        assert_eq!(lib.calling_convention, CallingConvention::C);
        assert!(lib.safety_checks);
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