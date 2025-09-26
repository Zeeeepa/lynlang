#!/usr/bin/env python3
import os
import re
import sys

def fix_hashmap_test(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Check if it needs fixing
    if 'HashMap' not in content or 'HashMap.new(' not in content:
        return False
    
    if 'get_default_allocator' in content:
        print(f"Already fixed: {filepath}")
        return False
    
    # Fix imports - add get_default_allocator if not present
    if '{ io' in content and 'get_default_allocator' not in content:
        content = re.sub(
            r'\{ io([^}]*)\}',
            lambda m: '{ io' + m.group(1) + ', get_default_allocator }',
            content
        )
    
    # Fix HashMap.new calls without allocator
    content = re.sub(
        r'HashMap\.new\(\)',
        'HashMap.new(alloc)',
        content
    )
    
    # Add allocator initialization before first HashMap.new
    lines = content.split('\n')
    new_lines = []
    added_alloc = False
    
    for line in lines:
        if not added_alloc and 'HashMap.new(alloc' in line:
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
        if 'hashmap' in filename.lower() and filename.endswith('.zen'):
            filepath = os.path.join(test_dir, filename)
            if fix_hashmap_test(filepath):
                fixed_count += 1
    
    print(f"\nFixed {fixed_count} test files")

if __name__ == '__main__':
    main()