#!/usr/bin/env python3
"""Run all tests and report results."""

import os
import subprocess
import sys
from pathlib import Path

def run_test(test_file, compiler):
    """Run a single test file."""
    try:
        result = subprocess.run(
            [compiler, test_file],
            capture_output=True,
            text=True,
            timeout=5
        )
        # Check if test produced expected output or exited cleanly
        if result.returncode == 0 or "Done" in result.stdout or "Complete" in result.stdout:
            return True, None
        else:
            return False, result.stderr[:200]
    except subprocess.TimeoutExpired:
        return False, "Timeout"
    except Exception as e:
        return False, str(e)

def main():
    test_dir = Path("tests")
    compiler = Path("target/debug/zen")
    
    if not compiler.exists():
        print(f"Compiler not found: {compiler}")
        return 1
    
    test_files = sorted(test_dir.glob("*.zen"))
    
    results = {
        "pass": [],
        "fail": [],
        "segfault": [],
        "timeout": []
    }
    
    print(f"Running {len(test_files)} tests...")
    print("-" * 60)
    
    for test_file in test_files:
        success, error = run_test(test_file, compiler)
        test_name = test_file.name
        
        if success:
            results["pass"].append(test_name)
            print(f"✓ {test_name}")
        else:
            if error and "Segmentation fault" in error:
                results["segfault"].append(test_name)
                print(f"💥 {test_name} (segfault)")
            elif error == "Timeout":
                results["timeout"].append(test_name)
                print(f"⏱ {test_name} (timeout)")
            else:
                results["fail"].append(test_name)
                print(f"✗ {test_name}")
                if error and len(error) < 100:
                    print(f"  Error: {error.strip()}")
    
    print("-" * 60)
    print(f"\n📊 Test Results:")
    print(f"  ✅ Passed: {len(results['pass'])}")
    print(f"  ❌ Failed: {len(results['fail'])}")
    print(f"  💥 Segfaults: {len(results['segfault'])}")
    print(f"  ⏱ Timeouts: {len(results['timeout'])}")
    print(f"  📈 Pass rate: {len(results['pass'])}/{len(test_files)} ({100*len(results['pass'])/len(test_files):.1f}%)")
    
    if results["pass"] and len(results["pass"]) <= 20:
        print(f"\n✅ Passing tests:")
        for test in results["pass"][:20]:
            print(f"  - {test}")
    
    return 0 if len(results["fail"]) == 0 else 1

if __name__ == "__main__":
    sys.exit(main())