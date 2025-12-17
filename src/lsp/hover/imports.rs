// Import-related hover functionality

use lsp_types::Position;

/// Information about an import
pub struct ImportInfo {
    pub import_line: String,
    pub source: String,
}

/// Find import information for a symbol
pub fn find_import_info(
    content: &str,
    symbol_name: &str,
    position: Position,
) -> Option<ImportInfo> {
    let lines: Vec<&str> = content.lines().collect();

    // Search backwards from current position for import statements
    let start_line = (position.line as usize).min(lines.len().saturating_sub(1));
    for i in (0..=start_line).rev() {
        let line = lines[i].trim();

        // Look for pattern: { symbol_name } = @std or { symbol_name, ... } = @std
        if line.starts_with('{') && line.contains('}') && line.contains('=') {
            // Extract the import pattern
            if let Some(brace_end) = line.find('}') {
                let import_part = &line[1..brace_end];
                let imports: Vec<&str> = import_part.split(',').map(|s| s.trim()).collect();

                if imports.contains(&symbol_name) {
                    // Extract the source (after =)
                    if let Some(eq_pos) = line.find('=') {
                        let source = line[eq_pos + 1..].trim();
                        return Some(ImportInfo {
                            import_line: line.to_string(),
                            source: source.to_string(),
                        });
                    }
                }
            }
        }
    }

    None
}
