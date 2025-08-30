#!/usr/bin/env python3
"""Fix loop syntax in Zen files to match current parser requirements."""

import re
import sys
from pathlib import Path

def fix_loop_syntax(content):
    """Convert loop (condition) { ... } to loop { condition ? | false => { break } | true => {} ... }"""
    
    # Pattern to match loop (condition) {
    pattern = r'(\s*)loop\s*\(([^)]+)\)\s*\{'
    
    def replacement(match):
        indent = match.group(1)
        condition = match.group(2).strip()
        
        # Special case for loop (true) - just use loop {
        if condition == "true":
            return f"{indent}loop {{"
        
        # For other conditions, convert to conditional break
        return f"{indent}loop {{\n{indent}    {condition} ? | false => {{ break }} | true => {{}}"
    
    return re.sub(pattern, replacement, content)

def process_file(filepath):
    """Process a single Zen file."""
    try:
        with open(filepath, 'r') as f:
            content = f.read()
        
        original = content
        fixed = fix_loop_syntax(content)
        
        if fixed != original:
            with open(filepath, 'w') as f:
                f.write(fixed)
            print(f"Fixed: {filepath}")
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return False

def main():
    """Main function."""
    stdlib_path = Path("/home/ubuntu/zenlang/stdlib")
    
    fixed_count = 0
    for zen_file in stdlib_path.glob("**/*.zen"):
        if process_file(zen_file):
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} files")

if __name__ == "__main__":
    main()