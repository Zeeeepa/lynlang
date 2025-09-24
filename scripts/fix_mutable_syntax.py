#!/usr/bin/env python3
"""Fix incorrect mutable assignment syntax in test files."""

import os
import re
import sys

def fix_mutable_assignment(content):
    """Replace := with ::= for mutable assignment, but not in other contexts."""
    # Pattern to match := that's not part of ::= 
    # Look for := that is not preceded by : and not in specific contexts
    lines = content.split('\n')
    fixed_lines = []
    
    for line in lines:
        # Skip comments
        if '//' in line:
            comment_start = line.index('//')
            code_part = line[:comment_start]
            comment_part = line[comment_start:]
        else:
            code_part = line
            comment_part = ''
        
        # Replace := with ::= when it's a mutable assignment
        # Look for patterns like "variable := value"
        pattern = r'(\w+)\s*:=\s*([^:=])'
        replacement = r'\1 ::= \2'
        fixed_code = re.sub(pattern, replacement, code_part)
        
        fixed_lines.append(fixed_code + comment_part)
    
    return '\n'.join(fixed_lines)

def main():
    test_dir = 'tests'
    if not os.path.exists(test_dir):
        print(f"Error: {test_dir} directory not found")
        return 1
    
    zen_files = [f for f in os.listdir(test_dir) if f.endswith('.zen')]
    
    fixed_count = 0
    for filename in zen_files:
        filepath = os.path.join(test_dir, filename)
        
        with open(filepath, 'r') as f:
            content = f.read()
        
        if ':=' in content and '::=' not in content:
            fixed_content = fix_mutable_assignment(content)
            
            if fixed_content != content:
                with open(filepath, 'w') as f:
                    f.write(fixed_content)
                print(f"Fixed: {filename}")
                fixed_count += 1
    
    print(f"\nFixed {fixed_count} files")
    return 0

if __name__ == '__main__':
    sys.exit(main())