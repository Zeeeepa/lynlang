//! Indexing logic for workspace and stdlib - extracted from document_store.rs

use super::symbol_extraction::extract_symbols_static;
use super::types::SymbolInfo;
use lsp_types::Url;
use std::collections::HashMap;
use std::path::Path;

/// Index workspace files recursively
pub fn index_workspace_files_recursive(path: &Path, symbols: &mut HashMap<String, SymbolInfo>) {
    use std::fs;

    if !path.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();

            if entry_path.is_file() && entry_path.extension().map_or(false, |e| e == "zen") {
                if let Ok(content) = fs::read_to_string(&entry_path) {
                    let file_path_str = entry_path.to_string_lossy();
                    let file_symbols = extract_symbols_static(&content, Some(&file_path_str));

                    // Convert path to URI for symbols
                    if let Ok(uri) = Url::from_file_path(&entry_path) {
                        for (name, mut symbol) in file_symbols {
                            symbol.definition_uri = Some(uri.clone());
                            symbols.insert(name, symbol);
                        }
                    }
                }
            } else if entry_path.is_dir() {
                let file_name = entry_path.file_name().and_then(|n| n.to_str());

                // Skip hidden directories and common build/cache directories
                if let Some(name) = file_name {
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        continue;
                    }
                }

                // Recursively index subdirectories
                index_workspace_files_recursive(&entry_path, symbols);
            }
        }
    }
}

/// Index stdlib directory recursively
pub fn index_stdlib_directory(path: &Path, symbols: &mut HashMap<String, SymbolInfo>) {
    use std::fs;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();

            if entry_path.is_file() && entry_path.extension().map_or(false, |e| e == "zen") {
                if let Ok(content) = fs::read_to_string(&entry_path) {
                    let file_path_str = entry_path.to_string_lossy();
                    let file_symbols = extract_symbols_static(&content, Some(&file_path_str));

                    if let Ok(uri) = Url::from_file_path(&entry_path) {
                        for (name, mut symbol) in file_symbols {
                            symbol.definition_uri = Some(uri.clone());
                            symbols.insert(name, symbol);
                        }
                    }
                }
            } else if entry_path.is_dir() {
                index_stdlib_directory(&entry_path, symbols);
            }
        }
    }
}

/// Find stdlib directory from common locations
pub fn find_stdlib_path() -> Option<std::path::PathBuf> {
    // Check environment variable first
    if let Ok(path) = std::env::var("ZEN_STDLIB_PATH") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // Try relative paths
    let stdlib_paths = [
        std::path::PathBuf::from("./stdlib"),
        std::path::PathBuf::from("../stdlib"),
        std::path::PathBuf::from("../../stdlib"),
    ];

    for stdlib_path in stdlib_paths {
        if stdlib_path.exists() {
            return Some(stdlib_path);
        }
    }
    None
}
