#!/bin/bash
echo "=========================================="
echo "QUICK LSP FEATURE CHECK"
echo "=========================================="
echo ""

echo "Test 1: Rename Symbol..."
python3 test_rename_feature.py >/dev/null 2>&1 && echo "✅ PASS" || echo "❌ FAIL"

echo "Test 2: Signature Help..."
python3 test_signature_help_feature.py >/dev/null 2>&1 && echo "✅ PASS" || echo "❌ FAIL"

echo "Test 3: Inlay Hints..."
python3 test_inlay_hints_feature.py >/dev/null 2>&1 && echo "✅ PASS" || echo "❌ FAIL"

echo "Test 4: Hover Types..."
python3 test_hover_types.py >/dev/null 2>&1 && echo "✅ PASS" || echo "❌ FAIL"

echo ""
echo "=========================================="
echo "FEATURE SUMMARY"
echo "=========================================="
