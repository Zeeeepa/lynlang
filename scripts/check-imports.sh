#!/bin/bash
# Centralized import syntax checker for Zen files
# Verifies that comptime imports follow the correct syntax

set -e

# Check for incorrect comptime import usage
# Correct:   comptime @std.option
# Incorrect: comptime { @std.option }
if grep -r "comptime.*{.*@std" --include="*.zen" . 2>/dev/null | grep -v "^Binary"; then
  echo "Error: Found incorrect comptime import usage!"
  echo ""
  echo "Imports must be at the module level, not inside blocks:"
  echo "  Correct:   comptime @std.option"
  echo "  Incorrect: comptime { @std.option }"
  exit 1
fi

echo "✓ Import syntax is correct"
exit 0
