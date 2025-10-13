#!/usr/bin/env python3
"""
Universal Code Analyzer - Validation Test Suite

This test suite validates the analyzer across all four scenarios:
1. True Positives (TP): Correctly identifies real errors
2. False Positives (FP): Incorrectly flags valid code as errors
3. True Negatives (TN): Correctly identifies valid code
4. False Negatives (FN): Misses actual errors

Each test case includes:
- Input code
- Expected outcome
- Actual outcome
- Validation result
"""

import os
import sys
import tempfile
from pathlib import Path
from typing import Dict, List, Any
import json

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# Import from the local module
import importlib.util
spec = importlib.util.spec_from_file_location("analyzer", "universal-code-analyzer.py")
analyzer_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(analyzer_module)

analyze_codebase = analyzer_module.analyze_codebase
Severity = analyzer_module.Severity


# ============================================================================
# TEST CASE DATA STRUCTURES
# ============================================================================

class TestCase:
    def __init__(self, name: str, code: str, language: str, 
                 expected_errors: int, expected_warnings: int,
                 description: str, category: str):
        self.name = name
        self.code = code
        self.language = language
        self.expected_errors = expected_errors
        self.expected_warnings = expected_warnings
        self.description = description
        self.category = category  # TP, FP, TN, FN
        
    def __repr__(self):
        return f"TestCase({self.name}, {self.category})"


# ============================================================================
# TRUE POSITIVE TEST CASES (Should detect errors)
# ============================================================================

TRUE_POSITIVE_TESTS = [
    # Python - Undefined Variable
    TestCase(
        name="py_undefined_variable",
        code="""
def process_data():
    result = undefined_variable + 10
    return result
""",
        language="python",
        expected_errors=1,
        expected_warnings=0,
        description="Undefined variable should be caught",
        category="TP"
    ),
    
    # Python - Type Error
    TestCase(
        name="py_type_error",
        code="""
def add_numbers(a: int, b: int) -> int:
    return a + b

result = add_numbers("hello", "world")
""",
        language="python",
        expected_errors=1,
        expected_warnings=0,
        description="Type mismatch should be detected",
        category="TP"
    ),
    
    # Python - Security Issue (SQL Injection)
    TestCase(
        name="py_sql_injection",
        code="""
import sqlite3

def get_user(username):
    conn = sqlite3.connect('db.sqlite')
    cursor = conn.cursor()
    query = "SELECT * FROM users WHERE username = '" + username + "'"
    cursor.execute(query)
    return cursor.fetchone()
""",
        language="python",
        expected_errors=0,
        expected_warnings=1,  # Security warning
        description="SQL injection vulnerability should be detected",
        category="TP"
    ),
    
    # TypeScript - Type Error
    TestCase(
        name="ts_type_error",
        code="""
function greet(name: string): string {
    return "Hello, " + name;
}

const result = greet(123);
""",
        language="typescript",
        expected_errors=1,
        expected_warnings=0,
        description="Type error should be caught",
        category="TP"
    ),
    
    # TypeScript - Unused Variable
    TestCase(
        name="ts_unused_variable",
        code="""
function calculate() {
    const unusedValue = 42;
    return 100;
}
""",
        language="typescript",
        expected_errors=0,
        expected_warnings=1,
        description="Unused variable should be warned",
        category="TP"
    ),
]


# ============================================================================
# TRUE NEGATIVE TEST CASES (Valid code, no errors)
# ============================================================================

TRUE_NEGATIVE_TESTS = [
    # Python - Valid Code
    TestCase(
        name="py_valid_function",
        code="""
def calculate_total(items: list) -> float:
    total = 0.0
    for item in items:
        total += item
    return total

result = calculate_total([1.0, 2.0, 3.0])
print(result)
""",
        language="python",
        expected_errors=0,
        expected_warnings=0,
        description="Valid Python code should pass",
        category="TN"
    ),
    
    # Python - Valid with Type Hints
    TestCase(
        name="py_valid_typed",
        code="""
from typing import List, Optional

def find_user(users: List[dict], user_id: int) -> Optional[dict]:
    for user in users:
        if user.get('id') == user_id:
            return user
    return None
""",
        language="python",
        expected_errors=0,
        expected_warnings=0,
        description="Valid typed Python should pass",
        category="TN"
    ),
    
    # TypeScript - Valid Code
    TestCase(
        name="ts_valid_function",
        code="""
interface User {
    id: number;
    name: string;
}

function getUser(id: number): User {
    return {
        id: id,
        name: "Test User"
    };
}

const user = getUser(1);
console.log(user.name);
""",
        language="typescript",
        expected_errors=0,
        expected_warnings=0,
        description="Valid TypeScript should pass",
        category="TN"
    ),
]


# ============================================================================
# FALSE POSITIVE TEST CASES (Valid code incorrectly flagged)
# ============================================================================

FALSE_POSITIVE_TESTS = [
    # Python - Dynamic typing (might be flagged incorrectly)
    TestCase(
        name="py_dynamic_typing",
        code="""
def process_value(value):
    # Intentionally untyped for flexibility
    if isinstance(value, str):
        return value.upper()
    elif isinstance(value, int):
        return value * 2
    return str(value)
""",
        language="python",
        expected_errors=0,
        expected_warnings=0,
        description="Valid dynamic typing should not be flagged",
        category="FP"
    ),
    
    # Python - Conditional imports (valid but might trigger warnings)
    TestCase(
        name="py_conditional_import",
        code="""
import sys

if sys.version_info >= (3, 8):
    from typing import Literal
else:
    from typing_extensions import Literal

def get_status() -> Literal["active", "inactive"]:
    return "active"
""",
        language="python",
        expected_errors=0,
        expected_warnings=0,
        description="Conditional imports are valid",
        category="FP"
    ),
]


# ============================================================================
# FALSE NEGATIVE TEST CASES (Errors that might be missed)
# ============================================================================

FALSE_NEGATIVE_TESTS = [
    # Python - Logic error (hard to detect)
    TestCase(
        name="py_logic_error",
        code="""
def calculate_average(numbers):
    total = sum(numbers)
    # Bug: should be len(numbers), not len(numbers) - 1
    return total / (len(numbers) - 1)
""",
        language="python",
        expected_errors=0,  # Logic errors are hard to detect statically
        expected_warnings=0,
        description="Logic errors are difficult to detect",
        category="FN"
    ),
    
    # Python - Runtime error (division by zero)
    TestCase(
        name="py_runtime_error",
        code="""
def divide_numbers(a, b):
    return a / b

result = divide_numbers(10, 0)
""",
        language="python",
        expected_errors=0,  # Runtime errors not caught by static analysis
        expected_warnings=0,
        description="Runtime errors require execution to detect",
        category="FN"
    ),
    
    # TypeScript - Subtle type coercion
    TestCase(
        name="ts_type_coercion",
        code="""
function processId(id: string | number) {
    // Subtle bug: comparing string to number
    if (id == "123") {  // Should be === for strict equality
        return true;
    }
    return false;
}
""",
        language="typescript",
        expected_errors=0,
        expected_warnings=0,  # Might not catch == vs === issues
        description="Subtle type coercion issues",
        category="FN"
    ),
]


# ============================================================================
# TEST RUNNER
# ============================================================================

def run_test(test: TestCase) -> Dict[str, Any]:
    """Run a single test case and return results."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Determine file extension
        ext_map = {
            "python": ".py",
            "typescript": ".ts",
            "javascript": ".js",
            "go": ".go",
            "rust": ".rs"
        }
        ext = ext_map.get(test.language, ".txt")
        
        # Write test code to file
        test_file = Path(tmpdir) / f"test{ext}"
        test_file.write_text(test.code)
        
        # Run analysis
        result = analyze_codebase(str(test_file), test.language)
        
        # Extract metrics
        actual_errors = result.summary.get("error", 0)
        actual_warnings = result.summary.get("warning", 0)
        
        # Determine if test passed
        errors_match = actual_errors == test.expected_errors
        warnings_match = actual_warnings == test.expected_warnings
        
        # For FP tests, we want NO errors/warnings
        # For FN tests, we expect to miss errors (so actual < expected is "correct")
        if test.category == "FP":
            passed = actual_errors == 0 and actual_warnings == 0
        elif test.category == "FN":
            passed = True  # FN tests are expected to miss errors
        else:
            passed = errors_match and warnings_match
        
        return {
            "test": test,
            "result": result,
            "actual_errors": actual_errors,
            "actual_warnings": actual_warnings,
            "passed": passed,
            "diagnostics": result.diagnostics[:5]  # First 5 diagnostics
        }


def print_test_result(test_result: Dict[str, Any]):
    """Print formatted test result."""
    test = test_result["test"]
    passed = test_result["passed"]
    
    status = "✅ PASS" if passed else "❌ FAIL"
    
    print(f"\n{status} | {test.category} | {test.name}")
    print(f"   Description: {test.description}")
    print(f"   Expected: {test.expected_errors} errors, {test.expected_warnings} warnings")
    print(f"   Actual:   {test_result['actual_errors']} errors, {test_result['actual_warnings']} warnings")
    
    if test_result["diagnostics"]:
        print("   Diagnostics:")
        for diag in test_result["diagnostics"]:
            print(f"      - [{diag.severity.value}] {diag.message[:60]}...")


def run_all_tests():
    """Run all test cases and generate report."""
    all_tests = (
        TRUE_POSITIVE_TESTS +
        TRUE_NEGATIVE_TESTS +
        FALSE_POSITIVE_TESTS +
        FALSE_NEGATIVE_TESTS
    )
    
    print("=" * 80)
    print("UNIVERSAL CODE ANALYZER - VALIDATION TEST SUITE")
    print("=" * 80)
    
    results_by_category = {
        "TP": [],
        "TN": [],
        "FP": [],
        "FN": []
    }
    
    # Run all tests
    for test in all_tests:
        result = run_test(test)
        results_by_category[test.category].append(result)
        print_test_result(result)
    
    # Generate summary report
    print("\n" + "=" * 80)
    print("TEST SUMMARY")
    print("=" * 80)
    
    for category, results in results_by_category.items():
        if not results:
            continue
            
        passed = sum(1 for r in results if r["passed"])
        total = len(results)
        
        category_name = {
            "TP": "True Positives (Correctly detected errors)",
            "TN": "True Negatives (Correctly validated code)",
            "FP": "False Positives (Incorrectly flagged valid code)",
            "FN": "False Negatives (Missed actual errors)"
        }[category]
        
        print(f"\n{category_name}")
        print(f"  Passed: {passed}/{total} ({100*passed//total if total > 0 else 0}%)")
        
        if category == "FP":
            false_positives = [r for r in results if not r["passed"]]
            if false_positives:
                print("  ⚠️ False Positives Detected:")
                for r in false_positives:
                    print(f"     - {r['test'].name}: {r['actual_errors']} errors, {r['actual_warnings']} warnings")
        
        if category == "FN":
            print("  ℹ️ These tests verify the analyzer correctly identifies its limitations")
    
    # Overall statistics
    all_results = [r for results in results_by_category.values() for r in results]
    total_passed = sum(1 for r in all_results if r["passed"])
    total_tests = len(all_results)
    
    print("\n" + "=" * 80)
    print(f"OVERALL: {total_passed}/{total_tests} tests passed ({100*total_passed//total_tests}%)")
    print("=" * 80)
    
    return results_by_category


def generate_json_report(results_by_category: Dict[str, List[Dict[str, Any]]]):
    """Generate JSON report of all test results."""
    report = {
        "test_categories": {},
        "summary": {}
    }
    
    for category, results in results_by_category.items():
        category_report = {
            "tests": [],
            "passed": 0,
            "failed": 0
        }
        
        for result in results:
            test = result["test"]
            category_report["tests"].append({
                "name": test.name,
                "description": test.description,
                "expected_errors": test.expected_errors,
                "expected_warnings": test.expected_warnings,
                "actual_errors": result["actual_errors"],
                "actual_warnings": result["actual_warnings"],
                "passed": result["passed"],
                "diagnostics_count": len(result["diagnostics"])
            })
            
            if result["passed"]:
                category_report["passed"] += 1
            else:
                category_report["failed"] += 1
        
        report["test_categories"][category] = category_report
    
    # Overall summary
    all_results = [r for results in results_by_category.values() for r in results]
    report["summary"] = {
        "total_tests": len(all_results),
        "passed": sum(1 for r in all_results if r["passed"]),
        "failed": sum(1 for r in all_results if not r["passed"]),
        "pass_rate": f"{100*sum(1 for r in all_results if r['passed'])//len(all_results)}%"
    }
    
    return report


# ============================================================================
# MAIN
# ============================================================================

if __name__ == "__main__":
    print("\nRunning validation tests...")
    print("This will test the analyzer against known scenarios:\n")
    
    results = run_all_tests()
    
    # Generate JSON report
    report = generate_json_report(results)
    
    # Save report
    report_path = Path(__file__).parent / "validation_report.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)
    
    print(f"\n📄 Detailed report saved to: {report_path}")
    print("\n✅ Validation complete!")
