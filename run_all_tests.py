#!/usr/bin/env python3
"""Run all .zen test files and categorize results"""

import subprocess
import os
from collections import defaultdict

os.chdir("/home/ubuntu/zenlang/tests")

tests = [f for f in os.listdir(".") if f.endswith(".zen") and not f.startswith("lsp_") and not f.startswith("test_diagnostics") and not f.startswith("test_inferred_types")]
results = {"passed": [], "parse_error": [], "ice": [], "runtime_error": [], "type_error": [], "other": []}

for i, test in enumerate(sorted(tests)):
    if (i + 1) % 50 == 0:
        print(f"Progress: {i+1}/{len(tests)}")

    try:
        result = subprocess.run(
            ["/home/ubuntu/zenlang/target/release/zen", test],
            capture_output=True,
            text=True,
            timeout=2
        )

        if result.returncode == 0:
            results["passed"].append(test)
        else:
            output = result.stdout + result.stderr

            if "Parse error" in output or "ParseError" in output:
                results["parse_error"].append(test)
            elif "Internal Compiler Error" in output or "ICE" in output or "panicked" in output:
                results["ice"].append(test)
            elif "Type mismatch" in output or "type error" in output or "Expected" in output:
                results["type_error"].append(test)
            elif result.returncode < 0:
                results["runtime_error"].append(test)
            else:
                results["other"].append(test)

    except subprocess.TimeoutExpired:
        results["runtime_error"].append(test)
    except Exception as e:
        results["other"].append(test)

# Print results
total = len(tests)
passed = len(results["passed"])
print(f"\n{'='*50}")
print(f"Test Results: {passed}/{total} passing ({100*passed//total}%)")
print(f"{'='*50}\n")

for category, tests_list in results.items():
    if tests_list and category != "passed":
        print(f"{category.upper().replace('_', ' ')}: {len(tests_list)} tests")
        for t in tests_list[:10]:  # Show first 10
            print(f"  - {t}")
        if len(tests_list) > 10:
            print(f"  ... and {len(tests_list) - 10} more")
        print()

# Save full results
with open("test_results.txt", "w") as f:
    f.write(f"Passed ({len(results['passed'])}):\n")
    for t in sorted(results["passed"]):
        f.write(f"  {t}\n")
    f.write("\n")

    for category in ["parse_error", "ice", "runtime_error", "type_error", "other"]:
        if results[category]:
            f.write(f"\n{category.upper()} ({len(results[category])}):\n")
            for t in sorted(results[category]):
                f.write(f"  {t}\n")

print(f"\nFull results saved to test_results.txt")
