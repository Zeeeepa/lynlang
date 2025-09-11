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
    lazy_loading: bool,
    version_requirement: Option<String>,
    search_paths: Vec<PathBuf>,
    aliases: HashMap<String, String>,
    error_handler: Option<Arc<dyn Fn(&FFIError) + Send + Sync>>,
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
            lazy_loading: false,
            version_requirement: None,
            search_paths: Self::default_search_paths(),
            aliases: HashMap::new(),
            error_handler: None,
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

    /// Enable lazy loading of symbols
    pub fn lazy_loading(mut self, enabled: bool) -> Self {
        self.lazy_loading = enabled;
        self
    }

    /// Set version requirement for the library
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version_requirement = Some(version.into());
        self
    }

    /// Add a search path for the library
    pub fn search_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.search_paths.push(path.into());
        self
    }

    /// Add multiple search paths
    pub fn search_paths(mut self, paths: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        self.search_paths.extend(paths.into_iter().map(|p| p.into()));
        self
    }

    /// Add a function alias
    pub fn alias(mut self, zen_name: impl Into<String>, c_name: impl Into<String>) -> Self {
        self.aliases.insert(zen_name.into(), c_name.into());
        self
    }

    /// Set custom error handler
    pub fn error_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&FFIError) + Send + Sync + 'static,
    {
        self.error_handler = Some(Arc::new(handler));
        self
    }

    /// Add struct definition for FFI
    pub fn struct_def(mut self, name: impl Into<String>, fields: Vec<(String, AstType)>) -> Self {
        let struct_type = AstType::Struct {
            name: Some(name.into()),
            fields: fields.into_iter().map(|(name, ty)| {
                crate::ast::StructField {
                    name: crate::ast::StructFieldName::Named(name),
                    ty,
                    mutable: false,
                    default_value: None,
                }
            }).collect(),
        };
        self.type_mappings.insert(
            name.into(), 
            TypeMapping {
                c_type: name.into(),
                zen_type: struct_type,
                marshaller: None,
            }
        );
        self
    }

    /// Add enum definition for FFI
    pub fn enum_def(mut self, name: impl Into<String>, variants: Vec<String>) -> Self {
        let enum_type = AstType::Enum {
            name: Some(name.into()),
            variants: variants.into_iter().map(|v| {
                crate::ast::EnumVariant {
                    name: v,
                    fields: vec![],
                }
            }).collect(),
        };
        self.type_mappings.insert(
            name.into(),
            TypeMapping {
                c_type: name.into(),
                zen_type: enum_type,
                marshaller: None,
            }
        );
        self
    }

    /// Build the library handle
    pub fn build(self) -> Result<Library, FFIError> {
        let path = self.path.unwrap_or_else(|| {
            self.find_library().unwrap_or_else(|| Self::default_lib_path(&self.name))
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
            lazy_loading: self.lazy_loading,
            version_requirement: self.version_requirement,
            aliases: self.aliases,
            error_handler: self.error_handler,
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

    /// Get default search paths based on platform
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        #[cfg(target_os = "linux")]
        {
            paths.push(PathBuf::from("/usr/lib"));
            paths.push(PathBuf::from("/usr/local/lib"));
            paths.push(PathBuf::from("/lib"));
            if let Ok(ld_path) = std::env::var("LD_LIBRARY_PATH") {
                for path in ld_path.split(':') {
                    paths.push(PathBuf::from(path));
                }
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            paths.push(PathBuf::from("/usr/lib"));
            paths.push(PathBuf::from("/usr/local/lib"));
            paths.push(PathBuf::from("/opt/homebrew/lib"));
            if let Ok(dyld_path) = std::env::var("DYLD_LIBRARY_PATH") {
                for path in dyld_path.split(':') {
                    paths.push(PathBuf::from(path));
                }
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            paths.push(PathBuf::from("C:\\Windows\\System32"));
            if let Ok(lib_path) = std::env::var("PATH") {
                for path in lib_path.split(';') {
                    paths.push(PathBuf::from(path));
                }
            }
        }
        
        paths
    }

    /// Find library in search paths
    fn find_library(&self) -> Option<PathBuf> {
        let lib_name = Self::default_lib_path(&self.name);
        
        for search_path in &self.search_paths {
            let full_path = search_path.join(&lib_name);
            if full_path.exists() {
                return Some(full_path);
            }
        }
        
        None
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
    lazy_loading: bool,
    version_requirement: Option<String>,
    aliases: HashMap<String, String>,
    error_handler: Option<Arc<dyn Fn(&FFIError) + Send + Sync>>,
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
        // Check for aliases
        let actual_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);
        
        let handle = self.handle.as_ref()
            .ok_or(FFIError::LibraryNotLoaded)?;

        // Verify function signature exists
        let signature = self.functions.get(actual_name)
            .ok_or_else(|| FFIError::SymbolNotFound(actual_name.to_string()))?;

        // Perform safety checks if enabled
        if self.safety_checks {
            self.validate_function_safety(name, signature)?;
        }

        // Load the symbol
        let _symbol_name = CString::new(name)
            .map_err(|_| FFIError::InvalidSymbolName(name.to_string()))?;

        unsafe {
            match handle.get::<*mut c_void>(actual_name.as_bytes()) {
                Ok(sym) => {
                    // Record successful function lookup
                    if let Ok(mut stats) = self.call_stats.lock() {
                        stats.record_call(actual_name, true);
                    }
                    Ok(*sym)
                },
                Err(_e) => {
                    let error = FFIError::SymbolNotFound(actual_name.to_string());
                    // Call error handler if present
                    if let Some(handler) = &self.error_handler {
                        handler(&error);
                    }
                    // Record failed function lookup
                    if let Ok(mut stats) = self.call_stats.lock() {
                        stats.record_call(actual_name, false);
                    }
                    Err(error)
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
    fn test_ffi_builder_enhanced() {
        // Test enhanced builder features
        let lib = FFI::lib("custom_lib")
            .version("1.2.3")
            .lazy_loading(true)
            .search_path("/custom/path")
            .alias("zen_func", "c_func_impl")
            .struct_def("Point", vec![
                ("x".to_string(), types::f64()),
                ("y".to_string(), types::f64()),
            ])
            .enum_def("Status", vec!["Ok".to_string(), "Error".to_string()])
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(lib.name, "custom_lib");
        assert!(lib.lazy_loading);
        assert_eq!(lib.version_requirement, Some("1.2.3".to_string()));
        assert!(lib.aliases.contains_key("zen_func"));
        assert_eq!(lib.aliases.get("zen_func"), Some(&"c_func_impl".to_string()));
        assert_eq!(lib.type_mappings.len(), 2);
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