#!/usr/bin/env python3
"""Fix duplicate @std imports in Zen test files."""

import os
import re

def fix_duplicate_imports(filepath):
    """Remove duplicate @std import lines."""
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    seen_imports = set()
    new_lines = []
    modified = False
    
    for line in lines:
        # Check if this is an @std import
        if re.match(r'^{.*} = @std', line):
            if line.strip() not in seen_imports:
                seen_imports.add(line.strip())
                new_lines.append(line)
            else:
                # Skip duplicate import
                modified = True
                print(f"  Removing duplicate: {line.strip()}")
        else:
            new_lines.append(line)
    
    if modified:
        with open(filepath, 'w') as f:
            f.writelines(new_lines)
        return True
    return False

def main():
    """Process all .zen files in tests directory."""
    fixed_count = 0
    
    for root, dirs, files in os.walk('tests'):
        for file in files:
            if file.endswith('.zen'):
                path = os.path.join(root, file)
                print(f"Checking {path}...")
                if fix_duplicate_imports(path):
                    fixed_count += 1
                    print(f"  Fixed!")
    
    print(f"\nFixed {fixed_count} files with duplicate imports")

if __name__ == "__main__":
    main()