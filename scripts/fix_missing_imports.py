#!/usr/bin/env python3
"""
Fix missing imports in test files.
"""

import os
import re

def fix_file(filepath):
    """Fix missing imports in a single file."""
    with open(filepath, 'r') as f:
        content = f.read()
    
    modified = False
    
    # Check what needs to be imported
    needs = set()
    if 'get_default_allocator()' in content:
        needs.add('get_default_allocator')
    if 'Array.new(' in content or 'Array<' in content:
        needs.add('Array')
    if 'HashMap<' in content:
        needs.add('HashMap')
    if 'DynVec<' in content or 'DynVec.new(' in content:
        needs.add('DynVec')
    if 'Option.' in content:
        needs.add('Option')
    if 'Result.' in content:
        needs.add('Result')
    
    if not needs:
        return False
    
    # Find the import line
    import_match = re.search(r'\{([^}]+)\}\s*=\s*@std', content)
    if import_match:
        current_imports = import_match.group(1)
        import_list = [item.strip() for item in current_imports.split(',')]
        
        # Add missing imports
        for item in needs:
            if item not in import_list:
                import_list.append(item)
                modified = True
        
        if modified:
            # Rebuild import line
            new_imports = ', '.join(import_list)
            new_line = f'{{{ new_imports }}} = @std'
            content = re.sub(r'\{[^}]+\}\s*=\s*@std', new_line, content)
            
            with open(filepath, 'w') as f:
                f.write(content)
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