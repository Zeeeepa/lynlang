#!/usr/bin/env python3
"""
Fix incorrect allocator syntax in test files.
Removes spurious ", get_default_allocator" before closing braces.
"""

import os
import re
import sys

def fix_file(filepath):
    """Fix allocator syntax in a single file."""
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Pattern to match incorrect syntax
    pattern = r',\s*get_default_allocator\s*\}'
    
    # Check if pattern exists
    if re.search(pattern, content):
        # Replace with just closing brace
        fixed_content = re.sub(pattern, '}', content)
        
        with open(filepath, 'w') as f:
            f.write(fixed_content)
        
        print(f"Fixed: {filepath}")
        return True
    return False

def main():
    test_dir = "tests"
    fixed_count = 0
    
    for filename in os.listdir(test_dir):
        if filename.endswith('.zen'):
            filepath = os.path.join(test_dir, filename)
            if fix_file(filepath):
                fixed_count += 1
    
    print(f"\nFixed {fixed_count} files")

if __name__ == "__main__":
    main()