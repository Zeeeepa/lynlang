//! Shared formatting utilities for Zen language
//! Used by both the LSP and zen-format CLI

/// Format enum variants to be on separate lines with proper indentation
/// Handles both inline enums and multi-line enums missing indentation.
pub fn format_enum_variants(content: &str) -> String {
    let mut result = String::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let leading_whitespace = &line[..line.len() - line.trim_start().len()];

        // Check for enum definition patterns
        if let Some(colon_pos) = trimmed.find(':') {
            let before_colon = trimmed[..colon_pos].trim();
            let after_colon = trimmed[colon_pos + 1..].trim();

            // Skip if not a valid identifier or contains assignment/struct markers
            if is_valid_identifier(before_colon)
                && !trimmed.contains('=')
                && !trimmed.contains('{')
                && !after_colon.starts_with(':')
            {
                // Case 1: Inline enum (Name: Variant1, Variant2, ...)
                if after_colon.contains(',') {
                    let variants: Vec<&str> = after_colon
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|v| !v.is_empty())
                        .collect();

                    let all_valid = variants.iter().all(|v| is_valid_variant(v));

                    if all_valid && variants.len() > 1 {
                        result.push_str(leading_whitespace);
                        result.push_str(before_colon);
                        result.push_str(":\n");

                        for (idx, variant) in variants.iter().enumerate() {
                            result.push_str(leading_whitespace);
                            result.push_str("    ");
                            result.push_str(variant);
                            if idx < variants.len() - 1 {
                                result.push(',');
                            }
                            result.push('\n');
                        }
                        i += 1;
                        continue;
                    }
                }

                // Case 2: Multi-line enum (Name: on its own, variants on following lines)
                if after_colon.is_empty() {
                    let mut variants = Vec::new();
                    let mut j = i + 1;

                    while j < lines.len() {
                        let next_line = lines[j];
                        let next_trimmed = next_line.trim();

                        if next_trimmed.is_empty() || next_trimmed.starts_with("//") {
                            break;
                        }

                        let variant_name = next_trimmed.trim_end_matches(',');
                        if is_valid_variant(variant_name) {
                            variants.push(variant_name);
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    if !variants.is_empty() {
                        result.push_str(leading_whitespace);
                        result.push_str(before_colon);
                        result.push_str(":\n");

                        for (idx, variant) in variants.iter().enumerate() {
                            result.push_str(leading_whitespace);
                            result.push_str("    ");
                            result.push_str(variant);
                            if idx < variants.len() - 1 {
                                result.push(',');
                            }
                            result.push('\n');
                        }

                        i = j;
                        continue;
                    }
                }
            }
        }

        result.push_str(line);
        result.push('\n');
        i += 1;
    }

    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Check if a string is a valid Zen identifier
pub fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Check if a string looks like a valid enum variant
/// Can be: `VariantName` or `VariantName(Type)` or `VariantName: Type`
pub fn is_valid_variant(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }

    // Variant with type annotation: `Ok: i32` or `Err: StaticString`
    if let Some(colon_pos) = s.find(':') {
        let name = s[..colon_pos].trim();
        let type_part = s[colon_pos + 1..].trim();
        return is_valid_identifier(name) && !type_part.is_empty();
    }

    // Variant with associated data in parens: `Some(i32)`
    if let Some(paren_pos) = s.find('(') {
        let name = &s[..paren_pos];
        return is_valid_identifier(name) && s.ends_with(')');
    }

    // Simple variant name: `Active`
    is_valid_identifier(s)
}

/// Remove trailing whitespace from all lines
pub fn remove_trailing_whitespace(content: &str) -> String {
    content
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalize variable declaration syntax
/// - `i: i32 ::= 0` → `i :: i32 = 0` (mutable with explicit type)
/// - `i: i32 = 0` stays as `i: i32 = 0` (immutable with explicit type)
/// - `i ::= 0` stays as `i ::= 0` (mutable with inferred type)
/// - `i = 0` stays as `i = 0` (immutable with inferred type)
pub fn normalize_variable_declarations(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            let leading_whitespace = &line[..line.len() - line.trim_start().len()];

            // Check for the problematic pattern: `name: Type ::= value`
            // This should become `name :: Type = value`
            if let Some(double_colon_eq) = trimmed.find("::=") {
                let before_dce = &trimmed[..double_colon_eq];
                let after_dce = &trimmed[double_colon_eq + 3..];

                // Check if there's a single colon before ::=
                if let Some(colon_pos) = before_dce.find(':') {
                    // Make sure it's not :: (double colon)
                    let after_first_colon = &before_dce[colon_pos..];
                    if !after_first_colon.starts_with("::") {
                        let name = before_dce[..colon_pos].trim();
                        let type_part = before_dce[colon_pos + 1..].trim();

                        // Only transform if we have a valid name and type
                        if !name.is_empty() && !type_part.is_empty() && is_valid_identifier(name) {
                            // Transform: `name: Type ::= value` → `name :: Type = value`
                            return format!("{}{} :: {} ={}", leading_whitespace, name, type_part, after_dce);
                        }
                    }
                }
            }

            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert tabs to 4 spaces
pub fn fix_indentation(content: &str) -> String {
    content.replace('\t', "    ")
}

/// Format based on braces and pattern matching
pub fn format_braces(content: &str) -> String {
    let mut formatted = String::new();
    let mut indent_level: usize = 0;
    let mut pattern_match_stack: Vec<usize> = Vec::new(); // stack of indent levels
    let indent_str = "    "; // 4 spaces

    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            formatted.push('\n');
            continue;
        }

        // Exit pattern matches before handling closing braces
        // We need to check this BEFORE decrementing for closing braces
        while let Some(&pm_indent) = pattern_match_stack.last() {
            // Exit pattern match if:
            // 1. Current line is not a pattern arm
            // 2. We're at the pattern match's indent level + 1 (inside the match)
            // 3. No more arms are coming
            let at_pattern_indent = indent_level == pm_indent + 1;
            let is_closing_brace = trimmed.starts_with('}') || trimmed.starts_with(']');

            if !trimmed.starts_with('|') && (at_pattern_indent || (is_closing_brace && indent_level == pm_indent + 2)) {
                // Check if there are more arms coming
                let more_arms = lines.iter().skip(i + 1)
                    .find(|l| !l.trim().is_empty())
                    .map(|l| l.trim().starts_with('|'))
                    .unwrap_or(false);

                if !more_arms {
                    pattern_match_stack.pop();
                    indent_level = indent_level.saturating_sub(1);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Decrease indent for lines starting with closing brace (after pattern match handling)
        if trimmed.starts_with('}') || trimmed.starts_with(']') {
            indent_level = indent_level.saturating_sub(1);
        }

        // Add indentation
        for _ in 0..indent_level {
            formatted.push_str(indent_str);
        }

        formatted.push_str(trimmed);
        formatted.push('\n');

        // Count braces for indent change (excluding leading close brace already handled)
        let opens = trimmed.matches('{').count() + trimmed.matches('[').count();
        let mut closes = trimmed.matches('}').count() + trimmed.matches(']').count();

        // Don't double-count leading close brace
        if trimmed.starts_with('}') || trimmed.starts_with(']') {
            closes = closes.saturating_sub(1);
        }

        // Update indent
        if opens > closes {
            indent_level += opens - closes;
        } else if closes > opens {
            indent_level = indent_level.saturating_sub(closes - opens);
        }

        // Start pattern match
        if trimmed.ends_with('?') {
            let has_arms = lines.iter().skip(i + 1)
                .find(|l| !l.trim().is_empty())
                .map(|l| l.trim().starts_with('|'))
                .unwrap_or(false);

            if has_arms {
                pattern_match_stack.push(indent_level);
                indent_level += 1;
            }
        }
    }

    formatted
}
