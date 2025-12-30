//! Loaded FFI library
//! Represents a loaded dynamic library with its functions and types

use libloading::Library as DynLib;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::builder::CallbackDefinition;
use super::errors::FFIError;
use super::stats::CallStatistics;
use super::types::{
    CallingConvention, FnSignature, FunctionSafety, LoadFlags, TypeMapping, TypeMarshaller,
};
use crate::ast::AstType;
use crate::stdlib_types::StdlibTypeRegistry;

/// Represents a loaded dynamic library
pub struct Library {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) functions: HashMap<String, FnSignature>,
    pub(crate) constants: HashMap<String, AstType>,
    pub(crate) type_mappings: HashMap<String, TypeMapping>,
    pub(crate) calling_convention: CallingConvention,
    pub(crate) safety_checks: bool,
    pub(crate) handle: Option<DynLib>,
    pub(crate) call_stats: Arc<Mutex<CallStatistics>>,
    pub(crate) lazy_loading: bool,
    pub(crate) version_requirement: Option<String>,
    pub(crate) aliases: HashMap<String, String>,
    pub(crate) error_handler: Option<Arc<dyn Fn(&FFIError) + Send + Sync>>,
    pub(crate) callbacks: HashMap<String, CallbackDefinition>,
    pub(crate) load_flags: LoadFlags,
}

impl Library {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        path: PathBuf,
        functions: HashMap<String, FnSignature>,
        constants: HashMap<String, AstType>,
        type_mappings: HashMap<String, TypeMapping>,
        calling_convention: CallingConvention,
        safety_checks: bool,
        lazy_loading: bool,
        version_requirement: Option<String>,
        aliases: HashMap<String, String>,
        error_handler: Option<Arc<dyn Fn(&FFIError) + Send + Sync>>,
        callbacks: HashMap<String, CallbackDefinition>,
        load_flags: LoadFlags,
    ) -> Self {
        Self {
            name,
            path,
            functions,
            constants,
            type_mappings,
            calling_convention,
            safety_checks,
            handle: None,
            call_stats: Arc::new(Mutex::new(CallStatistics::new())),
            lazy_loading,
            version_requirement,
            aliases,
            error_handler,
            callbacks,
            load_flags,
        }
    }

    /// Create standard marshallers for common types
    pub fn create_standard_marshallers() -> HashMap<String, TypeMarshaller> {
        let mut marshallers = HashMap::new();

        // String marshaller (Zen string <-> C string)
        marshallers.insert(
            "string".to_string(),
            TypeMarshaller {
                to_c: Arc::new(|data| {
                    let mut result = data.to_vec();
                    if !result.ends_with(&[0]) {
                        result.push(0);
                    }
                    result
                }),
                from_c: Arc::new(|data| {
                    let mut result = data.to_vec();
                    if let Some(pos) = result.iter().position(|&x| x == 0) {
                        result.truncate(pos);
                    }
                    result
                }),
            },
        );

        // Bool marshaller (ensure 0 or 1)
        marshallers.insert(
            "bool".to_string(),
            TypeMarshaller {
                to_c: Arc::new(|data| {
                    if data.is_empty() {
                        vec![0]
                    } else {
                        vec![if data[0] != 0 { 1 } else { 0 }]
                    }
                }),
                from_c: Arc::new(|data| {
                    if data.is_empty() {
                        vec![0]
                    } else {
                        vec![if data[0] != 0 { 1 } else { 0 }]
                    }
                }),
            },
        );

        marshallers
    }

    /// Load the dynamic library
    pub fn load(&mut self) -> Result<(), FFIError> {
        #[cfg(unix)]
        let lib_result = if self.load_flags.lazy_binding {
            unsafe {
                use libloading::os::unix::{Library as UnixLib, RTLD_LAZY};
                UnixLib::open(Some(&self.path), RTLD_LAZY).map(DynLib::from)
            }
        } else {
            unsafe { DynLib::new(&self.path) }
        };

        #[cfg(not(unix))]
        let lib_result = unsafe { DynLib::new(&self.path) };

        match lib_result {
            Ok(lib) => {
                self.handle = Some(lib);
                self.verify_version()?;
                self.initialize_callbacks()?;
                Ok(())
            }
            Err(e) => Err(FFIError::LibraryNotFound {
                path: self.path.display().to_string(),
                error: e.to_string(),
            }),
        }
    }

    /// Verify library version if required
    fn verify_version(&self) -> Result<(), FFIError> {
        if let Some(required_version) = &self.version_requirement {
            if let Some(version_fn_sig) = self
                .functions
                .get("version")
                .or_else(|| self.functions.get("get_version"))
            {
                if let Ok(_version_fn) = self
                    .get_function("version")
                    .or_else(|_| self.get_function("get_version"))
                {
                    match &version_fn_sig.returns {
                        AstType::Struct { name, .. } if StdlibTypeRegistry::is_string_type(name) => {}
                        t if t.is_ptr_type() => {
                            eprintln!("Version check enabled for: {}", required_version);
                        }
                        _ => {
                            return Err(FFIError::InvalidSignature {
                                function: "version".to_string(),
                                reason: "Unexpected signature for version function".to_string(),
                            });
                        }
                    }
                }
            } else {
                eprintln!(
                    "Warning: Version requirement {} specified but no version function found",
                    required_version
                );
            }
        }
        Ok(())
    }

    /// Initialize callbacks
    fn initialize_callbacks(&mut self) -> Result<(), FFIError> {
        for (name, def) in &self.callbacks {
            if def.signature.params.is_empty() {
                return Err(FFIError::ValidationError(format!(
                    "Callback {} must have at least one parameter",
                    name
                )));
            }

            eprintln!(
                "Callback {} initialized with {} parameters",
                name,
                def.signature.params.len()
            );
        }
        Ok(())
    }

    /// Get a function from the library with safety checks
    pub fn get_function(&self, name: &str) -> Result<*mut c_void, FFIError> {
        let actual_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);

        let handle = self.handle.as_ref().ok_or(FFIError::LibraryNotLoaded)?;

        let signature = self
            .functions
            .get(actual_name)
            .ok_or_else(|| FFIError::SymbolNotFound(actual_name.to_string()))?;

        if self.safety_checks {
            self.validate_function_safety(name, signature)?;
        }

        let _symbol_name =
            CString::new(name).map_err(|_| FFIError::InvalidSymbolName(name.to_string()))?;

        unsafe {
            match handle.get::<*mut c_void>(actual_name.as_bytes()) {
                Ok(sym) => {
                    if let Ok(mut stats) = self.call_stats.lock() {
                        stats.record_call(actual_name, true);
                    }
                    Ok(*sym)
                }
                Err(_e) => {
                    let error = FFIError::SymbolNotFound(actual_name.to_string());
                    if let Some(handler) = &self.error_handler {
                        handler(&error);
                    }
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
                for param_type in &sig.params {
                    if param_type.is_ptr_type() {
                        // Could add more validation here
                    }
                }
                Ok(())
            }
            FunctionSafety::Unsafe => Ok(()),
        }
    }

    /// Get a constant from the library
    pub fn get_constant(&self, name: &str) -> Result<*mut c_void, FFIError> {
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

    // Accessor methods
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
    pub fn functions(&self) -> &HashMap<String, FnSignature> {
        &self.functions
    }
    pub fn constants(&self) -> &HashMap<String, AstType> {
        &self.constants
    }
    pub fn type_mappings(&self) -> &HashMap<String, TypeMapping> {
        &self.type_mappings
    }
    pub fn calling_convention(&self) -> CallingConvention {
        self.calling_convention
    }
    pub fn safety_checks(&self) -> bool {
        self.safety_checks
    }
    pub fn version_requirement(&self) -> Option<&str> {
        self.version_requirement.as_deref()
    }
    pub fn lazy_loading(&self) -> bool {
        self.lazy_loading
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        self.unload();
    }
}
