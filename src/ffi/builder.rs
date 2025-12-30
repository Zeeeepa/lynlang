//! FFI library builder
//! Provides a builder pattern for configuring foreign libraries

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::errors::FFIError;
use super::library::Library;
use super::platform::{Platform, PlatformConfig};
use super::types::{CallingConvention, FnSignature, FunctionSafety, LoadFlags, TypeMapping};
use crate::ast::AstType;
use crate::stdlib_types::StdlibTypeRegistry;

/// Validation rule for FFI configuration
pub struct ValidationRule {
    pub name: String,
    pub validator: Arc<dyn Fn(&LibBuilder) -> Result<(), FFIError> + Send + Sync>,
}

impl ValidationRule {
    /// Execute the validation with the provided builder
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

/// Builder for configuring a foreign library
pub struct LibBuilder {
    pub(crate) name: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) functions: HashMap<String, FnSignature>,
    pub(crate) constants: HashMap<String, AstType>,
    pub(crate) type_mappings: HashMap<String, TypeMapping>,
    pub(crate) calling_convention: CallingConvention,
    pub(crate) safety_checks: bool,
    pub(crate) lazy_loading: bool,
    pub(crate) version_requirement: Option<String>,
    pub(crate) search_paths: Vec<PathBuf>,
    pub(crate) aliases: HashMap<String, String>,
    pub(crate) error_handler: Option<Arc<dyn Fn(&FFIError) + Send + Sync>>,
    pub(crate) callbacks: HashMap<String, CallbackDefinition>,
    pub(crate) platform_overrides: HashMap<Platform, PlatformConfig>,
    pub(crate) validation_rules: Vec<ValidationRule>,
    pub(crate) load_flags: LoadFlags,
}

impl LibBuilder {
    pub fn new(name: String) -> Self {
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
        let lib_name = lib_name.to_string();
        self.validation_rules.push(ValidationRule {
            name: format!("requires_{}", lib_name),
            validator: Arc::new(move |_builder| {
                let lib_path = Self::default_lib_path(&lib_name);
                let search_paths = Self::default_search_paths();

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
                zen_type: AstType::ptr(AstType::Void),
                marshaller: None,
            },
        );
        self
    }

    /// Add function with automatic signature inference from C header style
    pub fn function_from_c_decl(mut self, c_declaration: &str) -> Self {
        if let Some(sig) = Self::parse_c_function_decl(c_declaration) {
            if let Some(func_name) = Self::extract_function_name(c_declaration) {
                self.functions.insert(func_name, sig);
            }
        }
        self
    }

    /// Helper to parse C function declaration
    fn parse_c_function_decl(decl: &str) -> Option<FnSignature> {
        use super::type_helpers;

        let returns = if decl.starts_with("void ") {
            type_helpers::void()
        } else if decl.starts_with("int ") {
            type_helpers::i32()
        } else if decl.starts_with("char *") || decl.starts_with("const char *") {
            type_helpers::c_string()
        } else if decl.starts_with("double ") {
            type_helpers::f64()
        } else if decl.starts_with("float ") {
            type_helpers::f32()
        } else {
            type_helpers::raw_ptr(type_helpers::void())
        };

        let params = if decl.contains("(void)") || decl.contains("()") {
            vec![]
        } else if decl.contains("const char *") {
            vec![type_helpers::c_string()]
        } else {
            vec![type_helpers::raw_ptr(type_helpers::void())]
        };

        Some(FnSignature::new(params, returns).with_safety(FunctionSafety::Unsafe))
    }

    /// Extract function name from C declaration
    fn extract_function_name(decl: &str) -> Option<String> {
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

        Ok(Library::new(
            self.name,
            path,
            self.functions,
            self.constants,
            self.type_mappings,
            self.calling_convention,
            self.safety_checks,
            self.lazy_loading,
            self.version_requirement,
            self.aliases,
            self.error_handler,
            self.callbacks,
            self.load_flags,
        ))
    }

    /// Determine default library path based on platform
    pub(crate) fn default_lib_path(name: &str) -> PathBuf {
        #[cfg(target_os = "linux")]
        let filename = format!("lib{}.so", name);

        #[cfg(target_os = "macos")]
        let filename = format!("lib{}.dylib", name);

        #[cfg(target_os = "windows")]
        let filename = format!("{}.dll", name);

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let filename = format!("lib{}.so", name);

        PathBuf::from(filename)
    }

    /// Get default search paths based on platform
    pub(crate) fn default_search_paths() -> Vec<PathBuf> {
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
        match &mapping.zen_type {
            AstType::Struct { fields, .. } => {
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
            t if t.is_ptr_type() => true,
            AstType::Struct { name, .. } if StdlibTypeRegistry::is_string_type(name) => true,
            AstType::Array { .. } => true,
            AstType::FixedArray { .. } => true,
            AstType::FunctionPointer { .. } => true,
            AstType::Struct { name, .. } | AstType::Enum { name, .. } => {
                self.type_mappings.contains_key(name)
            }
            _ => false,
        }
    }
}
