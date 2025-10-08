#!/bin/bash
# Comprehensive LSP Feature Check
# Tests all major features that were listed as "missing" for 100% parity

echo "============================================================"
echo "COMPREHENSIVE LSP FEATURE CHECK"
echo "Testing 4 features that were needed for 100% parity"
echo "============================================================"
echo ""

PASS=0
FAIL=0

run_test() {
    local name="$1"
    local cmd="$2"
    
    echo -n "Testing $name... "
    if $cmd >/dev/null 2>&1; then
        echo "✅ PASS"
        PASS=$((PASS + 1))
    else
        echo "❌ FAIL"
        FAIL=$((FAIL + 1))
    fi
}

# Test the 4 critical features
run_test "Rename Symbol" "python3 tests/lsp/test_rename_feature.py"
run_test "Signature Help" "python3 tests/lsp/test_signature_help_feature.py"
run_test "Inlay Hints" "python3 tests/lsp/test_inlay_hints_feature.py"
run_test "Hover Types" "python3 tests/lsp/test_hover_types.py"

echo ""
echo "============================================================"
echo "RESULTS: $PASS/$((PASS + FAIL)) features working"
echo "============================================================"

if [ $FAIL -eq 0 ]; then
    echo "🎉 100% FEATURE PARITY CONFIRMED!"
    exit 0
else
    echo "⚠️  $FAIL feature(s) need work"
    exit 1
fi
