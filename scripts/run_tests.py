#!/usr/bin/env python3
"""
Unified Zen Test Runner
Runs all .zen test files and categorizes results
"""

import subprocess
import sys
from pathlib import Path
from collections import defaultdict


def categorize_failure(output, exit_code):
    """Categorize test failure based on output and exit code"""
    if "Parse error" in output or "ParseError" in output:
        return "parse_error"
    elif "Internal Compiler Error" in output or "ICE" in output or "panicked" in output:
        return "ice"
    elif "Type mismatch" in output or "type error" in output or "Expected" in output:
        return "type_error"
    elif exit_code == 124 or exit_code == -9:  # Timeout or killed
        return "timeout"
    elif exit_code < 0:  # Segfault or signal
        return "segfault"
    else:
        return "other"


def run_test(test_file, compiler_path, verbose=False):
    """Run a single test file and return (passed, category, error_msg)"""
    # Skip tests with intentional errors (used for LSP diagnostic testing)
    skip_tests = {"test_diagnostics.zen", "test_inferred_types.zen"}
    if test_file.name in skip_tests:
        return True, None, None

    try:
        result = subprocess.run(
            [compiler_path, str(test_file)],
            capture_output=True,
            text=True,
            timeout=5
        )

        # Filter out DEBUG lines from stderr
        stderr_lines = [line for line in result.stderr.splitlines()
                       if not line.startswith("DEBUG")]
        stderr = "\n".join(stderr_lines)
        output = result.stdout + stderr

        # List of tests that expect specific non-zero exit codes
        expected_nonzero = {
            "test_imports_basic.zen": 42,
            "test_import_simple_option.zen": 42,
            "test_math_import.zen": 10,
            "test_min_max.zen": 45,
            "test_simple_assign.zen": 10,
            "test_direct_min.zen": 10,
            "test_lsp.zen": 52,
            "test_lsp_simple.zen": 52,
            "test_dynvec_manual.zen": 42,
            "test_dynvec_pattern.zen": 42,
            "test_simple_get.zen": 42,
            "test_std_imports_simple.zen": 42,
            "test_find_references.zen": 100,
            "test_scope_rename.zen": 100,
        }

        # Check for compilation errors (these always fail regardless of exit code)
        if "error:" in stderr.lower() or "panic" in stderr.lower():
            category = categorize_failure(output, result.returncode)
            return False, category, f"Compilation failed: {stderr[:200]}"

        # Check expected exit code
        test_name = test_file.name
        if test_name in expected_nonzero:
            expected = expected_nonzero[test_name]
            if result.returncode != expected:
                return False, "wrong_exit_code", f"Expected exit {expected}, got {result.returncode}"
        else:
            # Default: expect exit code 0
            if result.returncode != 0:
                category = categorize_failure(output, result.returncode)
                return False, category, f"Runtime error (code {result.returncode})"

        return True, None, None

    except subprocess.TimeoutExpired:
        return False, "timeout", "Test timed out (>5s)"
    except Exception as e:
        return False, "other", str(e)


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Run Zen language test suite")
    parser.add_argument("--verbose", "-v", action="store_true",
                       help="Show detailed error messages")
    parser.add_argument("--categorize", "-c", action="store_true",
                       help="Show categorized failure breakdown")
    parser.add_argument("--compiler", default="./target/release/zen",
                       help="Path to compiler binary")
    parser.add_argument("test_dir", nargs="?", default="tests",
                       help="Directory containing test files")

    args = parser.parse_args()

    # Check compiler exists
    compiler_path = Path(args.compiler)
    if not compiler_path.exists():
        print(f"Error: Compiler not found at {compiler_path}")
        print("Building compiler...")
        result = subprocess.run(["cargo", "build", "--release"])
        if result.returncode != 0:
            print("Failed to build compiler")
            sys.exit(1)

    test_dir = Path(args.test_dir)
    if not test_dir.exists():
        print(f"Error: Test directory not found: {test_dir}")
        sys.exit(1)

    test_files = sorted(test_dir.glob("*.zen"))
    if not test_files:
        print(f"No test files found in {test_dir}")
        sys.exit(1)

    print(f"Running {len(test_files)} tests...")
    print()

    passed = 0
    failed = 0
    results_by_category = defaultdict(list)

    for i, test_file in enumerate(test_files, 1):
        success, category, error = run_test(test_file, compiler_path, args.verbose)

        if success:
            passed += 1
            if not args.categorize:
                print(f"✅ {test_file.name}")
        else:
            failed += 1
            results_by_category[category].append((test_file.name, error))
            if not args.categorize:
                # Show failures immediately if not categorizing
                if category == "segfault":
                    print(f"💥 {test_file.name} - SEGFAULT")
                else:
                    error_msg = error[:100] if error else category
                    print(f"❌ {test_file.name} - {error_msg}")

        # Progress indicator for large test suites
        if args.categorize and (i % 50 == 0 or i == len(test_files)):
            print(f"Progress: {i}/{len(test_files)}")

    # Print summary
    print()
    print("=" * 60)
    percentage = 100 * passed / len(test_files) if test_files else 0
    print(f"Test Results: {passed}/{len(test_files)} passed ({percentage:.1f}%)")
    print(f"Failures: {failed}")
    print("=" * 60)

    # Print categorized results
    if args.categorize and results_by_category:
        print()
        category_names = {
            "parse_error": "Parse Errors",
            "ice": "Internal Compiler Errors",
            "type_error": "Type Errors",
            "segfault": "Segmentation Faults",
            "timeout": "Timeouts",
            "wrong_exit_code": "Wrong Exit Code",
            "other": "Other Errors"
        }

        for category in ["parse_error", "ice", "type_error", "segfault",
                        "timeout", "wrong_exit_code", "other"]:
            if category in results_by_category:
                tests = results_by_category[category]
                print(f"\n{category_names[category]}: {len(tests)} tests")
                for test_name, error in tests[:10]:  # Show first 10
                    if args.verbose and error:
                        print(f"  - {test_name}: {error[:150]}")
                    else:
                        print(f"  - {test_name}")
                if len(tests) > 10:
                    print(f"  ... and {len(tests) - 10} more")

    # Show detailed errors in verbose mode (non-categorize)
    if args.verbose and not args.categorize and results_by_category:
        print()
        print("=" * 60)
        print("Failed tests (detailed):")
        all_failures = [(name, err, cat) for cat, tests in results_by_category.items()
                       for name, err in tests]
        for name, error, category in all_failures[:10]:
            print(f"\n{name} ({category}):")
            print(f"  {error[:200]}")

    # Exit with error if any tests failed
    sys.exit(1 if failed > 0 else 0)


if __name__ == "__main__":
    main()
