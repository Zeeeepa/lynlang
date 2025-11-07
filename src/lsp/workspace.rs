// Workspace Module for Zen LSP
// Handles workspace file discovery and utilities

use std::fs;
use std::path::{Path, PathBuf};
use std::io;

// ============================================================================
// PUBLIC UTILITY FUNCTIONS
// ============================================================================

/// Find all .zen files in a workspace directory
pub fn find_zen_files_in_workspace(root_path: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let mut zen_files = Vec::new();
    collect_zen_files_recursive(root_path, &mut zen_files)?;
    Ok(zen_files)
}

/// Recursively collect all .zen files in a directory
pub fn collect_zen_files_recursive(path: &Path, zen_files: &mut Vec<PathBuf>) -> Result<(), io::Error> {
    if !path.is_dir() {
        return Ok(());
    }

    // Skip common directories we don't want to search
    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
        if dir_name == "target" || dir_name == "node_modules" || dir_name == ".git"
            || dir_name.starts_with('.') {
            return Ok(());
        }
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            collect_zen_files_recursive(&entry_path, zen_files)?;
        } else if let Some(ext) = entry_path.extension() {
            if ext == "zen" {
                zen_files.push(entry_path);
            }
        }
    }

    Ok(())
}

