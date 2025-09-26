#!/usr/bin/env python3
import os
import re
import sys

def fix_hashmap_test(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Check if it needs fixing
    if 'HashMap<' not in content:
        return False
    
    if 'get_default_allocator' in content:
        print(f"Already fixed: {filepath}")
        return False
    
    # Fix imports - add get_default_allocator if not present
    if '@std' in content and 'get_default_allocator' not in content:
        # Update the import line
        content = re.sub(
            r'\{([^}]*HashMap[^}]*)\}',
            lambda m: '{' + m.group(1) + ', get_default_allocator }',
            content
        )
    
    # Fix HashMap<K,V>() calls to use allocator
    content = re.sub(
        r'HashMap<([^>]+)>\(\)',
        lambda m: f'HashMap<{m.group(1)}>(alloc)',
        content
    )
    
    # Add allocator initialization before first HashMap creation
    lines = content.split('\n')
    new_lines = []
    added_alloc = False
    in_main = False
    
    for line in lines:
        if 'main = ()' in line:
            in_main = True
        
        if in_main and not added_alloc and 'HashMap<' in line and '(alloc)' in line:
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
    
    # Also fix other tests that use HashMap
    for filename in ['test_allocator_simple.zen', 'test_collections.zen', 
                     'test_generic_hashmap_option.zen', 'test_generics_ultimate_stress.zen']:
        if os.path.exists(os.path.join(test_dir, filename)):
            filepath = os.path.join(test_dir, filename)
            if fix_hashmap_test(filepath):
                fixed_count += 1
    
    print(f"\nFixed {fixed_count} test files")

if __name__ == '__main__':
    main()