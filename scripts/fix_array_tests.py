#!/usr/bin/env python3
import os
import re
import sys

def fix_array_test(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Check if it needs fixing
    if 'Array.new(' not in content:
        return False
    
    if 'get_default_allocator' in content:
        print(f"Already fixed: {filepath}")
        return False
        
    # Fix imports
    content = re.sub(
        r'\{ io, Array \}',
        '{ io, get_default_allocator }',
        content
    )
    
    # Fix Array.new calls with 2 args (capacity, default)
    content = re.sub(
        r'Array\.new\((\d+), ([^)]+)\)',
        lambda m: f'Array.new(alloc, {m.group(1)}, {m.group(2)})',
        content
    )
    
    # Fix Array.new calls with 1 arg (capacity)
    content = re.sub(
        r'Array\.new\((\d+)\)',
        lambda m: f'Array.new(alloc, {m.group(1)}, 0)',
        content
    )
    
    # Add allocator initialization before first Array.new
    lines = content.split('\n')
    new_lines = []
    added_alloc = False
    
    for line in lines:
        if not added_alloc and 'Array.new(alloc' in line:
            # Find the indentation
            indent = len(line) - len(line.lstrip())
            new_lines.append(' ' * indent + 'alloc = get_default_allocator()')
            added_alloc = True
        new_lines.append(line)
    
    content = '\n'.join(new_lines)
    
    with open(filepath, 'w') as f:
        f.write(content)
    
    print(f"Fixed: {filepath}")
    return True

def main():
    test_dir = 'tests/'
    fixed_count = 0
    
    for filename in sorted(os.listdir(test_dir)):
        if filename.startswith('test_array') and filename.endswith('.zen'):
            filepath = os.path.join(test_dir, filename)
            if fix_array_test(filepath):
                fixed_count += 1
    
    print(f"\nFixed {fixed_count} test files")

if __name__ == '__main__':
    main()