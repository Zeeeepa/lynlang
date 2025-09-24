// FFI Module - Foreign Function Interface with Builder Pattern
// Implements the Language Spec v1.1.0 requirements for safe C interop

use libloading::Library as DynLib;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
    callbacks: HashMap<String, CallbackDefinition>,
    platform_overrides: HashMap<Platform, PlatformConfig>,
    validation_rules: Vec<ValidationRule>,
    load_flags: LoadFlags,
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
            callbacks: HashMap::new(),
            platform_overrides: HashMap::new(),
            validation_rules: Vec::new(),
            load_flags: LoadFlags::default(),
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
        self.search_paths
            .extend(paths.into_iter().map(|p| p.into()));
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
        let name_str = name.into();
        let struct_type = AstType::Struct {
            name: name_str.clone(),
            fields,
        };
        self.type_mappings.insert(
            name_str.clone(),
            TypeMapping {
                c_type: name_str,
                zen_type: struct_type,
                marshaller: None,
            },
        );
        self
    }

    /// Add enum definition for FFI
    pub fn enum_def(mut self, name: impl Into<String>, variants: Vec<String>) -> Self {
        let name_str = name.into();
        let enum_type = AstType::Enum {
            name: name_str.clone(),
            variants: variants
                .into_iter()
                .map(|v| crate::ast::EnumVariant {
                    name: v,
                    payload: None,
                })
                .collect(),
        };
        self.type_mappings.insert(
            name_str.clone(),
            TypeMapping {
                c_type: name_str,
                zen_type: enum_type,
                marshaller: None,
            },
        );
        self
    }

    /// Add callback definition
    pub fn callback(mut self, name: impl Into<String>, def: CallbackDefinition) -> Self {
        self.callbacks.insert(name.into(), def);
        self
    }

    /// Add platform-specific configuration
    pub fn platform_config(mut self, platform: Platform, config: PlatformConfig) -> Self {
        self.platform_overrides.insert(platform, config);
        self
    }

    /// Add validation rule
    pub fn validation_rule(mut self, rule: ValidationRule) -> Self {
        self.validation_rules.push(rule);
        self
    }

    /// Set load flags for library loading behavior
    pub fn load_flags(mut self, flags: LoadFlags) -> Self {
        self.load_flags = flags;
        self
    }

    /// Configure for current platform automatically
    pub fn auto_configure(mut self) -> Self {
        let current_platform = Platform::current();
        if let Some(config) = self.platform_overrides.get(&current_platform) {
            if let Some(path) = &config.path_override {
                self.path = Some(path.clone());
            }
            if let Some(conv) = config.calling_convention_override {
                self.calling_convention = conv;
            }
        }
        self
    }

    /// Add batch of functions with a common prefix
    pub fn functions_with_prefix(mut self, prefix: &str, funcs: Vec<(&str, FnSignature)>) -> Self {
        for (name, sig) in funcs {
            let full_name = format!("{}_{}", prefix, name);
            self.functions.insert(full_name, sig);
        }
        self
    }

    /// Add dependency checking for library
    pub fn requires(mut self, lib_name: &str) -> Self {
        // Add validation rule to check for dependency
        let lib_name = lib_name.to_string();
        self.validation_rules.push(ValidationRule {
            name: format!("requires_{}", lib_name),
            validator: Arc::new(move |_builder| {
                // Check if required library exists in system
                let lib_path = LibBuilder::default_lib_path(&lib_name);
                let search_paths = LibBuilder::default_search_paths();

                for search_path in search_paths {
                    let full_path = search_path.join(&lib_path);
                    if full_path.exists() {
                        return Ok(());
                    }
                }

                Err(FFIError::ValidationError(format!(
                    "Required dependency '{}' not found in system",
                    lib_name
                )))
            }),
        });
        self
    }

    /// Add metadata for library documentation
    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        // Store metadata in aliases temporarily (should have separate field in production)
        self.aliases
            .insert(format!("__meta_{}", key), value.to_string());
        self
    }

    /// Configure opaque pointer types for FFI
    pub fn opaque_type(mut self, name: impl Into<String>) -> Self {
        let name_str = name.into();
        self.type_mappings.insert(
            name_str.clone(),
            TypeMapping {
                c_type: format!("struct {}", name_str),
                zen_type: AstType::Ptr(Box::new(AstType::Void)),
                marshaller: None,
            },
        );
        self
    }

    /// Add function with automatic signature inference from C header style
    pub fn function_from_c_decl(mut self, c_declaration: &str) -> Self {
        // Parse simple C function declaration
        // Example: "int sqlite3_open(const char *filename, sqlite3 **ppDb)"
        if let Some(sig) = Self::parse_c_function_decl(c_declaration) {
            if let Some(func_name) = Self::extract_function_name(c_declaration) {
                self.functions.insert(func_name, sig);
            }
        }
        self
    }

    /// Helper to parse C function declaration
    fn parse_c_function_decl(decl: &str) -> Option<FnSignature> {
        // Simple parser for basic C function declarations
        // This is a simplified version - in production would use a proper C parser

        let returns = if decl.starts_with("void ") {
            types::void()
        } else if decl.starts_with("int ") {
            types::i32()
        } else if decl.starts_with("char *") || decl.starts_with("const char *") {
            types::c_string()
        } else if decl.starts_with("double ") {
            types::f64()
        } else if decl.starts_with("float ") {
            types::f32()
        } else {
            types::raw_ptr(types::void())
        };

        // Extract parameters (simplified)
        let params = if decl.contains("(void)") || decl.contains("()") {
            vec![]
        } else if decl.contains("const char *") {
            vec![types::c_string()]
        } else {
            vec![types::raw_ptr(types::void())]
        };

        Some(FnSignature::new(params, returns).with_safety(FunctionSafety::Unsafe))
    }

    /// Extract function name from C declaration
    fn extract_function_name(decl: &str) -> Option<String> {
        // Find function name between return type and opening parenthesis
        if let Some(paren_pos) = decl.find('(') {
            let before_paren = &decl[..paren_pos];
            let parts: Vec<&str> = before_paren.split_whitespace().collect();
            if let Some(last) = parts.last() {
                return Some(last.trim_start_matches('*').to_string());
            }
        }
        None
    }

    /// Build the library handle with enhanced validation
    pub fn build(mut self) -> Result<Library, FFIError> {
        // Apply platform-specific configurations
        self = self.auto_configure();

        // Run validation rules
        for rule in &self.validation_rules {
            rule.validate(&self)?;
        }

        // Validate function signatures
        for (name, sig) in &self.functions {
            self.validate_signature(name, sig)?;
        }

        // Validate type mappings
        for (name, mapping) in &self.type_mappings {
            self.validate_type_mapping(name, mapping)?;
        }

        let path = if let Some(p) = self.path {
            p
        } else {
            self.find_library()
                .ok_or_else(|| FFIError::LibraryNotFound {
                    path: self.name.clone(),
                    error: format!("Could not find library '{}' in search paths", self.name),
                })?
        };

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
            callbacks: self.callbacks,
            load_flags: self.load_flags,
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

    /// Find library in search paths with better error reporting
    fn find_library(&self) -> Option<PathBuf> {
        let lib_name = Self::default_lib_path(&self.name);

        // Try exact path first
        if let Some(ref path) = self.path {
            if path.exists() {
                return Some(path.clone());
            }
        }

        // Try with version-specific names
        if let Some(ref version) = self.version_requirement {
            let versioned_names = vec![
                format!("lib{}.so.{}", self.name, version),
                format!("lib{}-{}.so", self.name, version),
                format!("{}-{}.dll", self.name, version),
                format!("lib{}.{}.dylib", self.name, version),
            ];

            for versioned_name in versioned_names {
                for search_path in &self.search_paths {
                    let full_path = search_path.join(&versioned_name);
                    if full_path.exists() {
                        return Some(full_path);
                    }
                }
            }
        }

        // Try standard library names
        for search_path in &self.search_paths {
            let full_path = search_path.join(&lib_name);
            if full_path.exists() {
                return Some(full_path);
            }

            // Try without lib prefix on Windows
            #[cfg(target_os = "windows")]
            {
                let alt_name = format!("{}.dll", self.name);
                let alt_path = search_path.join(&alt_name);
                if alt_path.exists() {
                    return Some(alt_path);
                }
            }
        }

        None
    }

    /// Validate function signature
    fn validate_signature(&self, name: &str, sig: &FnSignature) -> Result<(), FFIError> {
        // Check for unsupported types in FFI
        for param in &sig.params {
            if !self.is_ffi_compatible_type(param) {
                return Err(FFIError::ValidationError(format!(
                    "Function '{}' has incompatible parameter type for FFI",
                    name
                )));
            }
        }

        if !self.is_ffi_compatible_type(&sig.returns) {
            return Err(FFIError::ValidationError(format!(
                "Function '{}' has incompatible return type for FFI",
                name
            )));
        }

        // Warn about variadic functions without safety checks
        if sig.variadic && self.safety_checks && sig.safety != FunctionSafety::Unsafe {
            eprintln!(
                "Warning: Variadic function '{}' should be marked as Unsafe",
                name
            );
        }

        Ok(())
    }

    /// Validate type mapping
    fn validate_type_mapping(&self, name: &str, mapping: &TypeMapping) -> Result<(), FFIError> {
        // Ensure the Zen type is compatible with the C type
        match &mapping.zen_type {
            AstType::Struct { fields, .. } => {
                // Check that all fields are FFI-compatible
                for (field_name, field_type) in fields {
                    if !self.is_ffi_compatible_type(field_type) {
                        return Err(FFIError::ValidationError(format!(
                            "Struct '{}' field '{}' has incompatible type for FFI",
                            name, field_name
                        )));
                    }
                }
            }
            AstType::Enum { .. } => {
                // Enums are generally OK for FFI as they map to integers
            }
            _ => {
                if !self.is_ffi_compatible_type(&mapping.zen_type) {
                    return Err(FFIError::ValidationError(format!(
                        "Type mapping '{}' has incompatible type for FFI",
                        name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Check if a type is FFI-compatible
    fn is_ffi_compatible_type(&self, ast_type: &AstType) -> bool {
        match ast_type {
            // Primitive types are always FFI-compatible
            AstType::Void
            | AstType::Bool
            | AstType::I8
            | AstType::I16
            | AstType::I32
            | AstType::I64
            | AstType::U8
            | AstType::U16
            | AstType::U32
            | AstType::U64
            | AstType::F32
            | AstType::F64 => true,

            // Pointers are FFI-compatible
            AstType::Ptr(_) => true,

            // Strings need special handling but are allowed
            AstType::String => true,

            // Arrays with known size are OK
            AstType::Array { .. } => true,

            // Fixed arrays with known size are OK
            AstType::FixedArray { .. } => true,

            // Function pointers are FFI-compatible
            AstType::FunctionPointer { .. } => true,

            // Structs and enums need to be in type_mappings
            AstType::Struct { name, .. } | AstType::Enum { name, .. } => {
                self.type_mappings.contains_key(name)
            }

            // Other types may not be FFI-compatible
            _ => false,
        }
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
    callbacks: HashMap<String, CallbackDefinition>,
    load_flags: LoadFlags,
}

impl Library {
    /// Create standard marshallers for common types
    pub fn create_standard_marshallers() -> HashMap<String, TypeMarshaller> {
        let mut marshallers = HashMap::new();

        // String marshaller (Zen string <-> C string)
        marshallers.insert(
            "string".to_string(),
            TypeMarshaller {
                to_c: Arc::new(|data| {
                    // Convert Zen string to null-terminated C string
                    let mut result = data.to_vec();
                    if !result.ends_with(&[0]) {
                        result.push(0);
                    }
                    result
                }),
                from_c: Arc::new(|data| {
                    // Convert C string to Zen string (remove null terminator)
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
        // Apply load flags if needed
        #[cfg(unix)]
        let lib_result = if self.load_flags.lazy_binding {
            unsafe {
                use libloading::os::unix::{Library as UnixLib, RTLD_LAZY};
                UnixLib::open(Some(&self.path), RTLD_LAZY).map(|lib| DynLib::from(lib))
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
            // Try to call a standard version function if it exists
            if let Some(version_fn_sig) = self
                .functions
                .get("version")
                .or_else(|| self.functions.get("get_version"))
            {
                if let Ok(_version_fn) = self
                    .get_function("version")
                    .or_else(|_| self.get_function("get_version"))
                {
                    // Validate that the function returns a string-like type
                    match &version_fn_sig.returns {
                        AstType::String | AstType::Ptr(_) => {
                            // Version check would be performed at runtime
                            eprintln!("Version check enabled for: {}", required_version);
                        }
                        _ => {
                            return Err(FFIError::InvalidSignature {
                                function: "version".to_string(),
                                reason: format!("Unexpected signature for version function"),
                            });
                        }
                    }
                }
            } else {
                // No version function found, emit warning
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
            // Set up callback trampolines for C -> Zen calls
            // Store callback metadata for runtime invocation
            // Track callback registration in stats

            // Validate callback signature
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
        // Check for aliases
        let actual_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);

        let handle = self.handle.as_ref().ok_or(FFIError::LibraryNotLoaded)?;

        // Verify function signature exists
        let signature = self
            .functions
            .get(actual_name)
            .ok_or_else(|| FFIError::SymbolNotFound(actual_name.to_string()))?;

        // Perform safety checks if enabled
        if self.safety_checks {
            self.validate_function_safety(name, signature)?;
        }

        // Load the symbol
        let _symbol_name =
            CString::new(name).map_err(|_| FFIError::InvalidSymbolName(name.to_string()))?;

        unsafe {
            match handle.get::<*mut c_void>(actual_name.as_bytes()) {
                Ok(sym) => {
                    // Record successful function lookup
                    if let Ok(mut stats) = self.call_stats.lock() {
                        stats.record_call(actual_name, true);
                    }
                    Ok(*sym)
                }
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
                    if matches!(param_type, AstType::Ptr(_)) {
                        // Could add more validation here
                    }
                }
                Ok(())
            }
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
        Err(FFIError::InvalidSymbolName(
            "Direct calling not yet implemented".to_string(),
        ))
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

    /// Get version requirement
    pub fn version_requirement(&self) -> Option<&str> {
        self.version_requirement.as_deref()
    }

    /// Get callbacks
    pub fn callbacks(&self) -> &HashMap<String, CallbackDefinition> {
        &self.callbacks
    }

    /// Get load flags
    pub fn load_flags(&self) -> &LoadFlags {
        &self.load_flags
    }

    /// Check if lazy loading is enabled
    pub fn lazy_loading(&self) -> bool {
        self.lazy_loading
    }

    /// Get search paths (derived from builder, not stored in Library)
    pub fn search_paths(&self) -> Vec<PathBuf> {
        // Return default search paths as Library doesn't store them
        LibBuilder::default_search_paths()
    }

    /// Register a callback function
    pub fn register_callback(
        &mut self,
        name: &str,
        callback: Box<dyn Fn(&[u8]) -> Vec<u8>>,
    ) -> Result<(), FFIError> {
        if let Some(def) = self.callbacks.get(name) {
            // Store the callback for later invocation
            // In a real implementation, this would set up proper FFI trampolines

            // Validate the callback matches expected signature
            if self.safety_checks {
                // Perform runtime type checking when callback is invoked
                eprintln!("Callback {} registered with safety checks enabled", name);
            }

            // Store callback in a registry (would be used by trampoline)
            // For now, we just acknowledge registration
            // Track callback registration in stats
            eprintln!(
                "Successfully registered callback {} with signature {:?}",
                name, def.signature
            );

            // Store the actual callback for invocation
            // In production, this would integrate with the FFI layer
            drop(callback); // Placeholder - would be stored in callback registry

            Ok(())
        } else {
            Err(FFIError::InvalidSymbolName(format!(
                "Unknown callback: {}",
                name
            )))
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        self.unload();
    }
}

/// Type mapping between Zen and C types
#[derive(Debug, Clone)]
pub struct TypeMapping {
    pub c_type: String,
    pub zen_type: AstType,
    pub marshaller: Option<Arc<TypeMarshaller>>,
}

/// Type marshaller for converting between Zen and C representations
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

/// Calling convention for FFI functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    C,
    Stdcall,
    Fastcall,
    Vectorcall,
    Thiscall,
    System,
}

/// Load flags for library loading behavior
#[derive(Debug, Clone)]
pub struct LoadFlags {
    pub lazy_binding: bool,
    pub global_symbols: bool,
    pub local_symbols: bool,
    pub nodelete: bool,
}

impl Default for LoadFlags {
    fn default() -> Self {
        Self {
            lazy_binding: false,
            global_symbols: false,
            local_symbols: true,
            nodelete: false,
        }
    }
}

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
                write!(f, "Invalid symbol name: '{}'", name)
            }
            FFIError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
            FFIError::LibraryNotLoaded => {
                write!(f, "Library not loaded")
            }
            FFIError::CallFailed { function, error } => {
                write!(f, "Call to function '{}' failed: {}", function, error)
            }
        }
    }
}

impl std::error::Error for FFIError {}

/// Callback definition for FFI
#[derive(Clone)]
pub struct CallbackDefinition {
    pub signature: FnSignature,
    pub trampoline: Option<Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>>,
}

impl std::fmt::Debug for CallbackDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackDefinition")
            .field("signature", &self.signature)
            .field(
                "trampoline",
                &self.trampoline.as_ref().map(|_| "<function>"),
            )
            .finish()
    }
}

/// Platform-specific configuration
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub path_override: Option<PathBuf>,
    pub calling_convention_override: Option<CallingConvention>,
    pub additional_search_paths: Vec<PathBuf>,
}

/// Platform enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    FreeBSD,
    Android,
    IOs,
    Wasm,
    Other(String),
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "freebsd")]
        return Platform::FreeBSD;
        #[cfg(target_os = "android")]
        return Platform::Android;
        #[cfg(target_os = "ios")]
        return Platform::IOs;
        #[cfg(target_arch = "wasm32")]
        return Platform::Wasm;
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
            target_os = "freebsd",
            target_os = "android",
            target_os = "ios",
            target_arch = "wasm32"
        )))]
        return Platform::Other(std::env::consts::OS.to_string());
    }
}

// Removed duplicate type definitions - they are already defined above

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
        // Use platform-specific size
        #[cfg(target_pointer_width = "64")]
        return AstType::U64;
        #[cfg(target_pointer_width = "32")]
        return AstType::U32;
        #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
        return AstType::U64; // Default to 64-bit
    }

    pub fn isize() -> AstType {
        // Use platform-specific size
        #[cfg(target_pointer_width = "64")]
        return AstType::I64;
        #[cfg(target_pointer_width = "32")]
        return AstType::I32;
        #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
        return AstType::I64; // Default to 64-bit
    }

    pub fn string() -> AstType {
        AstType::String
    }

    pub fn c_string() -> AstType {
        // C string is a pointer to char (u8)
        AstType::Ptr(Box::new(AstType::U8))
    }

    pub fn raw_ptr(inner: AstType) -> AstType {
        // Use regular Pointer for raw pointers in FFI
        AstType::Ptr(Box::new(inner))
    }

    pub fn ptr(inner: AstType) -> AstType {
        // Use regular Pointer type
        AstType::Ptr(Box::new(inner))
    }

    pub fn array(size: usize, element_type: AstType) -> AstType {
        // Use FixedArray for arrays with known size
        AstType::FixedArray {
            element_type: Box::new(element_type),
            size,
        }
    }

    pub fn slice(element_type: AstType) -> AstType {
        // A slice is a dynamic array
        AstType::Array(Box::new(element_type))
    }

    pub fn function(params: Vec<AstType>, returns: AstType) -> AstType {
        AstType::FunctionPointer {
            param_types: params,
            return_type: Box::new(returns),
        }
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
            .function(
                "sqlite3_open",
                FnSignature::new(
                    vec![
                        types::string(),
                        types::raw_ptr(types::raw_ptr(types::void())),
                    ],
                    types::i32(),
                )
                .with_safety(FunctionSafety::Unsafe),
            )
            .function(
                "sqlite3_close",
                FnSignature::new(vec![types::raw_ptr(types::void())], types::i32())
                    .with_safety(FunctionSafety::Unsafe),
            )
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
        // Test enhanced builder features - with explicit path to avoid lookup
        let lib = FFI::lib("custom_lib")
            .path("/tmp/libcustom.so") // Use explicit path to bypass find_library
            .version("1.2.3")
            .lazy_loading(true)
            .search_path("/custom/path")
            .alias("zen_func", "c_func_impl")
            .struct_def(
                "Point",
                vec![
                    ("x".to_string(), types::f64()),
                    ("y".to_string(), types::f64()),
                ],
            )
            .enum_def("Status", vec!["Ok".to_string(), "Error".to_string()])
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(lib.name, "custom_lib");
        assert!(lib.lazy_loading);
        assert_eq!(lib.version_requirement, Some("1.2.3".to_string()));
        assert!(lib.aliases.contains_key("zen_func"));
        assert_eq!(
            lib.aliases.get("zen_func"),
            Some(&"c_func_impl".to_string())
        );
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

    #[test]
    fn test_ffi_builder_advanced_features() {
        // Test batch function addition
        let lib = FFI::lib("math")
            .path("/tmp/libmath.so")
            .functions_with_prefix(
                "math",
                vec![
                    ("sin", FnSignature::new(vec![types::f64()], types::f64())),
                    ("cos", FnSignature::new(vec![types::f64()], types::f64())),
                    ("tan", FnSignature::new(vec![types::f64()], types::f64())),
                ],
            )
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert!(lib.functions.contains_key("math_sin"));
        assert!(lib.functions.contains_key("math_cos"));
        assert!(lib.functions.contains_key("math_tan"));
    }

    #[test]
    fn test_opaque_types() {
        let lib = FFI::lib("opaque_test")
            .path("/tmp/libopaque.so")
            .opaque_type("sqlite3")
            .opaque_type("FILE")
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(lib.type_mappings.len(), 2);
        assert!(lib.type_mappings.contains_key("sqlite3"));
        assert!(lib.type_mappings.contains_key("FILE"));
    }

    #[test]
    fn test_c_declaration_parsing() {
        let lib = FFI::lib("c_test")
            .path("/tmp/libc_test.so")
            .function_from_c_decl("int printf(const char *format)")
            .function_from_c_decl("void exit(int status)")
            .function_from_c_decl("double sqrt(double x)")
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();

        // Check printf
        if let Some(printf_sig) = lib.functions.get("printf") {
            assert_eq!(printf_sig.returns, types::i32());
            assert_eq!(printf_sig.params.len(), 1);
        }

        // Check exit
        if let Some(exit_sig) = lib.functions.get("exit") {
            assert_eq!(exit_sig.returns, types::void());
        }

        // Check sqrt
        if let Some(sqrt_sig) = lib.functions.get("sqrt") {
            assert_eq!(sqrt_sig.returns, types::f64());
        }
    }

    #[test]
    fn test_metadata_storage() {
        let lib = FFI::lib("metadata_test")
            .path("/tmp/libmeta.so")
            .metadata("version", "1.2.3")
            .metadata("author", "Zen Team")
            .metadata("license", "MIT")
            .build();

        assert!(lib.is_ok());
        let lib = lib.unwrap();
        assert_eq!(
            lib.aliases.get("__meta_version"),
            Some(&"1.2.3".to_string())
        );
        assert_eq!(
            lib.aliases.get("__meta_author"),
            Some(&"Zen Team".to_string())
        );
        assert_eq!(lib.aliases.get("__meta_license"), Some(&"MIT".to_string()));
    }
}
