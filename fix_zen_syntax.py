#!/usr/bin/env python3
"""
Script to automatically fix Zen syntax patterns according to language spec.
Replaces if/else/match with the ? operator pattern.
"""
import re
import os
import sys
from pathlib import Path

def fix_if_else_pattern(content):
    """Replace if/else blocks with ? operator"""
    patterns = [
        # Simple if statement without else
        (r'if\s+([^{]+)\s*\{([^}]+)\}(?!\s*else)',
         r'\1 ? {\2}'),
        
        # if/else blocks
        (r'if\s+([^{]+)\s*\{([^}]+)\}\s*else\s*\{([^}]+)\}',
         r'\1 ? {\2} : {\3}'),
        
        # Nested if in else (else if)
        (r'if\s+([^{]+)\s*\{([^}]+)\}\s*else\s+if\s+([^{]+)\s*\{([^}]+)\}',
         r'\1 ? {\2} : \3 ? {\4}'),
    ]
    
    for pattern, replacement in patterns:
        content = re.sub(pattern, replacement, content, flags=re.MULTILINE | re.DOTALL)
    
    return content

def fix_match_pattern(content):
    """Replace match expressions with ? operator"""
    # This is more complex and needs careful handling
    # Basic match pattern
    pattern = r'match\s+([^{]+)\s*\{([^}]+)\}'
    
    def replace_match(match):
        expr = match.group(1).strip()
        body = match.group(2).strip()
        
        # Convert match arms to ? pattern
        result = f"{expr} ?\n"
        arms = re.findall(r'\|\s*([^=]+)=>\s*([^|]+?)(?=\||$)', body)
        for i, (pattern, action) in enumerate(arms):
            pattern = pattern.strip()
            action = action.strip()
            # Convert Ok/Err patterns
            if pattern.startswith('Ok(') or pattern.startswith('Err('):
                pattern = '.' + pattern.replace('(', ' -> ').replace(')', '')
            result += f"        | {pattern} => {action}\n"
        
        return result.rstrip()
    
    content = re.sub(pattern, replace_match, content, flags=re.MULTILINE | re.DOTALL)
    return content

def fix_zen_file(filepath):
    """Fix a single .zen file"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original = content
        
        # Apply fixes
        content = fix_if_else_pattern(content)
        content = fix_match_pattern(content)
        
        # Only write if changes were made
        if content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return False

def main():
    # Find all .zen files
    zen_files = list(Path('/home/ubuntu/zenlang').rglob('*.zen'))
    
    # Skip certain directories if needed
    skip_dirs = ['node_modules', '.git', 'build', 'out']
    zen_files = [f for f in zen_files if not any(skip in str(f) for skip in skip_dirs)]
    
    print(f"Found {len(zen_files)} .zen files to process")
    
    fixed_count = 0
    for filepath in zen_files:
        if fix_zen_file(filepath):
            fixed_count += 1
            print(f"Fixed: {filepath}")
    
    print(f"\nFixed {fixed_count} files")

if __name__ == "__main__":
    main()