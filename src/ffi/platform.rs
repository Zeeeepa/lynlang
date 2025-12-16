//! Platform-specific FFI configuration

use super::types::CallingConvention;
use std::path::PathBuf;

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
