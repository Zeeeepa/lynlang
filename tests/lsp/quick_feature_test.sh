#!/bin/bash
# Quick feature test for LSP

echo "================================"
echo "QUICK LSP FEATURE TEST"
echo "================================"

cd /home/ubuntu/zenlang

echo ""
echo "1. Testing Rename..."
timeout 5 python3 tests/lsp/test_rename_simple.py 2>&1 | grep -E "(SUCCESS|FAILED|edits)"

echo ""
echo "2. Testing Signature Help..."
timeout 5 python3 tests/lsp/test_signature_simple.py 2>&1 | grep -E "(SUCCESS|signature)"

echo ""
echo "3. Testing Inlay Hints..."
timeout 5 python3 tests/lsp/test_inlay_simple.py 2>&1 | grep -E "(SUCCESS|hint)"

echo ""
echo "4. Testing Hover..."
timeout 5 python3 tests/lsp/test_hover_types.py 2>&1 | grep -E "(PASSED|FAILED)"

echo ""
echo "================================"
echo "FEATURE TEST COMPLETE"
echo "================================"
