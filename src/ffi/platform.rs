// Platform-specific FFI configuration

use super::types::CallingConvention;
use std::path::PathBuf;

/// Platform-specific configuration
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub path_override: Option<PathBuf>,
    pub calling_convention_override: Option<CallingConvention>,
}

/// Platform enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Platform::Unknown;
    }
}

/// Callback definition for FFI
#[derive(Debug, Clone)]
pub struct CallbackDefinition {
    pub name: String,
    pub signature: super::types::FnSignature,
    pub handler: Option<String>, // Name of Zen function to call
}



