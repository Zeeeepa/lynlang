// Import-related helper functions

use lsp_types::Position;

pub struct ImportInfo {
    pub import_line: String,
    pub source: String,
}

pub fn find_import_info(
    content: &str,
    symbol_name: &str,
    position: Position,
) -> Option<ImportInfo> {
    let start_line = position.line as usize;
    let lines: Vec<&str> = content.lines().take(start_line + 1).collect();

    for (_line_num, line) in lines.iter().enumerate().rev() {
        let trimmed = line.trim();

        if trimmed.starts_with('{') && trimmed.contains('}') && trimmed.contains('=') {
            if let Some(brace_end) = trimmed.find('}') {
                let import_part = &trimmed[1..brace_end];
                let imports: Vec<&str> = import_part.split(',').map(|s| s.trim()).collect();

                if imports.contains(&symbol_name) {
                    if let Some(eq_pos) = trimmed.find('=') {
                        let source = trimmed[eq_pos + 1..].trim();
                        return Some(ImportInfo {
                            import_line: trimmed.to_string(),
                            source: source.to_string(),
                        });
                    }
                }
            }
        }
    }

    None
}
