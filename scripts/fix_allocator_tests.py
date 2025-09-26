#!/usr/bin/env python3
"""Fix allocator-related issues in test files."""

import re
import os
from pathlib import Path

def fix_array_new_calls(content):
    """Fix Array.new calls to include allocator."""
    # Pattern for Array.new(capacity, default)
    pattern = r'Array\.new\((\d+),\s*([^)]+)\)'
    
    # Check if get_default_allocator is already imported
    if 'get_default_allocator' not in content:
        # Add to imports if not present
        import_pattern = r'(\{[^}]+\}\s*=\s*@std)'
        def add_allocator_to_import(match):
            imports = match.group(1)
            if 'get_default_allocator' not in imports:
                # Insert get_default_allocator before the closing brace
                return imports[:-1] + ', get_default_allocator' + imports[-1:]
            return imports
        content = re.sub(import_pattern, add_allocator_to_import, content)
    
    # Check if allocator is already defined
    if 'alloc = get_default_allocator()' not in content:
        # Find main function and add allocator at the start
        main_pattern = r'(main\s*=\s*\([^)]*\)[^{]*\{)'
        def add_allocator_def(match):
            return match.group(1) + '\n    alloc = get_default_allocator()'
        content = re.sub(main_pattern, add_allocator_def, content, count=1)
    
    # Replace Array.new calls
    def fix_array_new(match):
        capacity = match.group(1)
        default = match.group(2)
        return f'Array.new(alloc, {capacity}, {default})'
    
    content = re.sub(pattern, fix_array_new, content)
    return content

def fix_hashmap_new_calls(content):
    """Fix HashMap constructor calls to include allocator."""
    # Pattern for HashMap<K,V>() without allocator
    pattern = r'HashMap<([^>]+)>\(\)'
    
    # Check if get_default_allocator is already imported
    if 'get_default_allocator' not in content:
        # Add to imports if not present
        import_pattern = r'(\{[^}]+\}\s*=\s*@std)'
        def add_allocator_to_import(match):
            imports = match.group(1)
            if 'get_default_allocator' not in imports:
                # Insert get_default_allocator before the closing brace
                return imports[:-1] + ', get_default_allocator' + imports[-1:]
            return imports
        content = re.sub(import_pattern, add_allocator_to_import, content)
    
    # Check if allocator is already defined
    if 'alloc = get_default_allocator()' not in content:
        # Find main function or test functions and add allocator
        func_pattern = r'((?:main|test_\w+)\s*=\s*\([^)]*\)[^{]*\{)'
        def add_allocator_def(match):
            func_start = match.group(1)
            # Check if this function already has alloc defined nearby
            next_lines = content[match.end():match.end()+100]
            if 'alloc' not in next_lines:
                return func_start + '\n    alloc = get_default_allocator()'
            return func_start
        content = re.sub(func_pattern, add_allocator_def, content)
    
    # Replace HashMap constructor calls
    def fix_hashmap_new(match):
        types = match.group(1)
        return f'HashMap<{types}>(alloc)'
    
    content = re.sub(pattern, fix_hashmap_new, content)
    return content

def process_file(filepath):
    """Process a single test file."""
    try:
        with open(filepath, 'r') as f:
            content = f.read()
        
        original = content
        
        # Apply fixes
        content = fix_array_new_calls(content)
        content = fix_hashmap_new_calls(content)
        
        # Write back if changed
        if content != original:
            with open(filepath, 'w') as f:
                f.write(content)
            print(f"Fixed: {filepath}")
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return False

def main():
    tests_dir = Path('/home/ubuntu/zenlang/tests')
    
    fixed_count = 0
    for test_file in tests_dir.glob('*.zen'):
        if process_file(test_file):
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} files")

if __name__ == '__main__':
    main()