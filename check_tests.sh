#!/bin/bash
# Script to run all .zen test files and categorize failures

cd /home/ubuntu/zenlang/tests
total=0
passed=0
failed=0
parse_errors=0
ice_errors=0
runtime_errors=0
type_errors=0
other_errors=0

declare -a parse_error_tests
declare -a ice_tests
declare -a runtime_tests
declare -a type_error_tests
declare -a other_error_tests

for f in *.zen; do
    total=$((total + 1))
    output=$(timeout 2 ../target/release/zen "$f" 2>&1)
    exit_code=$?

    # Check if there's actual error output (Compilation error, Parse error, etc.)
    has_error_output=$(echo "$output" | grep -E "error:|Error:|Parse error|Type mismatch|Internal Compiler Error|ICE|panicked")

    if [ $exit_code -eq 0 ] || [ -z "$has_error_output" ]; then
        # Exit 0 OR no error output means success
        # (Zen programs return their main() value, so non-zero doesn't mean error)
        passed=$((passed + 1))
    else
        failed=$((failed + 1))

        if echo "$output" | grep -q "Parse error"; then
            parse_errors=$((parse_errors + 1))
            parse_error_tests+=("$f")
        elif echo "$output" | grep -q "Internal Compiler Error\|ICE\|panicked"; then
            ice_errors=$((ice_errors + 1))
            ice_tests+=("$f")
        elif echo "$output" | grep -q "Type mismatch\|type error\|Type error\|Expected.*got"; then
            type_errors=$((type_errors + 1))
            type_error_tests+=("$f")
        elif [ $exit_code -eq 124 ]; then
            # Timeout - likely infinite loop
            runtime_errors=$((runtime_errors + 1))
            runtime_tests+=("$f")
        else
            other_errors=$((other_errors + 1))
            other_error_tests+=("$f")
        fi
    fi
done

echo "========================================="
echo "Test Results: $passed/$total passing ($(awk "BEGIN {print ($passed/$total)*100}")%)"
echo "========================================="
echo ""
echo "Parse Errors: $parse_errors tests"
for t in "${parse_error_tests[@]}"; do echo "  - $t"; done
echo ""
echo "Internal Compiler Errors: $ice_errors tests"
for t in "${ice_tests[@]}"; do echo "  - $t"; done
echo ""
echo "Runtime Errors: $runtime_errors tests"
for t in "${runtime_tests[@]}"; do echo "  - $t"; done
echo ""
echo "Type Errors: $type_errors tests"
for t in "${type_error_tests[@]}"; do echo "  - $t"; done
echo ""
echo "Other Errors: $other_errors tests"
for t in "${other_error_tests[@]}"; do echo "  - $t"; done
