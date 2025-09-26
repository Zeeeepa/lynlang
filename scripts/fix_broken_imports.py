#!/usr/bin/env python3
"""Fix broken imports that were mangled by the allocator script."""

import re
import os
from pathlib import Path

def fix_broken_imports(content):
    """Fix import lines that were broken by the allocator script."""
    
    # Pattern for broken imports like: {io, Array} = @st, get_default_allocatord
    broken_pattern1 = r'\{([^}]+)\}\s*=\s*@st,\s*get_default_allocatord'
    
    # Fix by reconstructing the import correctly
    def fix_import1(match):
        imports = match.group(1)
        # Add get_default_allocator to the imports if not already there
        if 'get_default_allocator' not in imports:
            imports = imports + ', get_default_allocator'
        return f'{{{imports}}} = @std'
    
    content = re.sub(broken_pattern1, fix_import1, content)
    
    # Pattern for other broken variants
    # Example: {io, Array, get_default_allocator} = @std followed by garbage
    broken_pattern2 = r'(\{[^}]+,\s*get_default_allocator\}\s*=\s*@std)[^}\n]+'
    content = re.sub(broken_pattern2, r'\1', content)
    
    # Pattern for doubled get_default_allocator
    doubled_pattern = r'get_default_allocator,\s*get_default_allocator'
    content = re.sub(doubled_pattern, 'get_default_allocator', content)
    
    # Fix missing @std (just has @st)
    content = re.sub(r'=\s*@st\b', '= @std', content)
    
    # Fix cases where get_default_allocator was appended incorrectly outside braces
    # Example: } = @std, get_default_allocator
    wrong_outside = r'(\}\s*=\s*@std),\s*get_default_allocator'
    
    def fix_outside(match):
        # Need to put get_default_allocator inside the braces
        # Find the opening brace and inject there
        before_match = content[:match.start()]
        if '{' in before_match:
            last_brace_idx = before_match.rfind('{')
            after_brace = before_match[last_brace_idx+1:]
            # Check if get_default_allocator already in imports
            if 'get_default_allocator' not in after_brace:
                # We'll handle this in a second pass
                pass
        return match.group(1)
    
    content = re.sub(wrong_outside, fix_outside, content)
    
    # More comprehensive fix: find all import lines and ensure they're correct
    import_line_pattern = r'^(\s*)(\{[^}]+\})\s*=\s*([^}\n]+)$'
    
    def fix_import_line(match):
        indent = match.group(1)
        imports = match.group(2)
        rest = match.group(3).strip()
        
        # Clean up the rest part
        if rest.startswith('@st'):
            rest = '@std'
        elif not rest.startswith('@std'):
            # Something's wrong, try to extract @std
            if '@st' in rest:
                rest = '@std'
            elif '@' not in rest:
                rest = '@std'
        else:
            rest = '@std'
        
        # Ensure get_default_allocator is in imports if needed
        # (Don't add it to every import, only if file uses allocators)
        
        return f"{indent}{imports} = {rest}"
    
    content = re.sub(import_line_pattern, fix_import_line, content, flags=re.MULTILINE)
    
    return content

def process_file(filepath):
    """Process a single test file."""
    try:
        with open(filepath, 'r') as f:
            content = f.read()
        
        original = content
        
        # Apply fixes
        content = fix_broken_imports(content)
        
        # Write back if changed
        if content != original:
            with open(filepath, 'w') as f:
                f.write(content)
            print(f"Fixed imports: {filepath}")
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