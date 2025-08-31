# Global Memory - Zen Import System Refactor

## Current Import Patterns

### Working (Keep as is):
- `core := @std.core` - Direct standard library imports
- `io := @std.io` - Direct standard library imports  
- `lexer := @compiler.lexer` - Compiler module imports

### Need to Fix:
- `build.import("io")` → Should be `@std.io`
- `build.import("fs")` → Should be `@std.fs`
- `build.import("custom")` → Need new syntax for user modules

## Key Insights:
1. Parser already rejects imports in comptime blocks ✓
2. Module-level imports with `:=` already work ✓
3. `build.import()` is deprecated, need to replace with direct `@std.` imports
4. Need to distinguish between:
   - Standard library: `@std.module`
   - Compiler modules: `@compiler.module`
   - User modules: Need new syntax (maybe `@user.module` or relative paths?)

## Files Using build.import() (need updating):
- tools/syntax_checker.zen
- tests/test_self_hosted_lexer.zen
- tests/test_import_system_comprehensive.zen
- tests/test_import_comprehensive.zen
- examples/*.zen (many files)

## Next Steps:
1. Replace all `build.import("module")` with `@std.module`
2. Ensure parser handles all import patterns correctly
3. Update documentation and examples