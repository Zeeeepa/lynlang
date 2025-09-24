#!/usr/bin/env python3
import subprocess
import os
import sys
from pathlib import Path

def run_test(test_file):
    """Run a single test file and return (passed, error_msg)"""
    try:
        # Compile and run the test directly
        result = subprocess.run(
            ["./target/release/zen", test_file],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        # Filter out DEBUG lines from stderr
        stderr_lines = [line for line in result.stderr.splitlines() if not line.startswith("DEBUG")]
        stderr = "\n".join(stderr_lines)
        
        # Check for compilation errors
        if "error:" in stderr.lower() or "panic" in stderr.lower():
            return False, f"Compilation failed: {stderr[:200]}"
        
        # Check for segfault
        if result.returncode == -11 or "Segmentation fault" in stderr:
            return False, "Segmentation fault"
        
        # Check exit code
        if result.returncode != 0:
            return False, f"Runtime error (code {result.returncode})"
        
        return True, None
            
    except subprocess.TimeoutExpired:
        return False, "Test timed out"
    except Exception as e:
        return False, str(e)

def main():
    test_dir = Path("tests")
    test_files = sorted(test_dir.glob("*.zen"))
    
    passed = 0
    failed = 0
    segfaults = 0
    results = []
    
    print(f"Running {len(test_files)} tests...")
    
    for test_file in test_files:
        success, error = run_test(test_file)
        
        if success:
            passed += 1
            print(f"✅ {test_file.name}")
        else:
            failed += 1
            if "Segmentation fault" in str(error):
                segfaults += 1
                print(f"💥 {test_file.name} - SEGFAULT")
            else:
                print(f"❌ {test_file.name} - {error[:100]}")
            results.append((test_file.name, error))
    
    print(f"\n{'='*60}")
    print(f"Test Results: {passed}/{len(test_files)} passed ({100*passed/len(test_files):.1f}%)")
    print(f"Failures: {failed} (including {segfaults} segfaults)")
    
    if results and "--verbose" in sys.argv:
        print(f"\n{'='*60}")
        print("Failed tests:")
        for name, error in results[:10]:  # Show first 10 failures
            print(f"\n{name}:")
            print(f"  {error[:200]}")

if __name__ == "__main__":
    main()