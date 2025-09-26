#!/usr/bin/env python3
"""
Fix HashMap.new() calls to use allocator constructor.
"""

import os
import re

def fix_file(filepath):
    """Fix HashMap constructor calls in a single file."""
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    modified = False
    new_lines = []
    needs_allocator = False
    
    for line in lines:
        # Check if this line has HashMap.new()
        if 'HashMap<' in line and '.new()' in line:
            # Replace .new() with allocator constructor
            new_line = re.sub(r'HashMap<([^>]+)>\.new\(\)', r'HashMap<\1>(alloc)', line)
            new_lines.append(new_line)
            needs_allocator = True
            modified = True
        else:
            new_lines.append(line)
    
    # Add allocator if needed
    if needs_allocator:
        for i, line in enumerate(new_lines):
            if 'main = ()' in line:
                # Insert allocator call after main function start
                j = i + 1
                while j < len(new_lines) and new_lines[j].strip() == '':
                    j += 1
                # Check if allocator already exists
                if 'get_default_allocator' not in ''.join(new_lines[j:j+3]):
                    new_lines.insert(j, '    alloc = get_default_allocator()\n')
                break
        
        # Check if get_default_allocator is imported
        for i, line in enumerate(new_lines):
            if '} = @std' in line:
                if 'get_default_allocator' not in line:
                    new_lines[i] = line.replace('} = @std', ', get_default_allocator } = @std')
                break
    
    if modified:
        with open(filepath, 'w') as f:
            f.writelines(new_lines)
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