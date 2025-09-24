#!/usr/bin/env python3

import os
import re
import sys

def fix_result_syntax(filepath):
    """Fix Ok/Err to use Result.Ok/Result.Err syntax"""
    with open(filepath, 'r') as f:
        content = f.read()
    
    original_content = content
    
    # Remove Ok and Err from imports
    content = re.sub(r'{ io, Result, Ok, Err }', '{ io, Result }', content)
    
    # Fix Ok() constructor calls - but not in patterns
    # Match Ok( not preceded by |
    content = re.sub(r'(?<![|])(\s+)Ok\(', r'\1Result.Ok(', content)
    content = re.sub(r'^Ok\(', 'Result.Ok(', content, flags=re.MULTILINE)
    content = re.sub(r'return Ok\(', 'return Result.Ok(', content)
    content = re.sub(r'= Ok\(', '= Result.Ok(', content)
    
    # Fix Err() constructor calls - but not in patterns  
    content = re.sub(r'(?<![|])(\s+)Err\(', r'\1Result.Err(', content)
    content = re.sub(r'^Err\(', 'Result.Err(', content, flags=re.MULTILINE)
    content = re.sub(r'return Err\(', 'return Result.Err(', content)
    content = re.sub(r'= Err\(', '= Result.Err(', content)
    
    # Fix pattern matches - add Result. prefix
    content = re.sub(r'\| Ok\(', '| Result.Ok(', content)
    content = re.sub(r'\| Err\(', '| Result.Err(', content)
    
    if content != original_content:
        with open(filepath, 'w') as f:
            f.write(content)
        return True
    return False

def main():
    test_dir = 'tests'
    fixed_count = 0
    
    for filename in os.listdir(test_dir):
        if filename.endswith('.zen'):
            filepath = os.path.join(test_dir, filename)
            if fix_result_syntax(filepath):
                fixed_count += 1
                print(f"Fixed: {filename}")
    
    print(f"\nFixed {fixed_count} files")

if __name__ == "__main__":
    main()