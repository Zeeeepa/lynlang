# Stdlib Parse/Type Errors - 2026-01-10

## Overview

Several stdlib files had parser and type checker errors. Most have been fixed.

---

## Issue 1: eventfd.zen - Bitwise OR in function call

**File:** `stdlib/io/eventfd.zen`
**Status:** ✅ FIXED

**Changes:**
- Added bitwise operators (`|`, `&`, `^`, `<<`, `>>`) to the parser and codegen
- Added `parse_pattern_expression` to avoid treating `|` as bitwise OR in pattern context
- Added `&` as prefix unary operator for address-of expressions
- Fixed expression parser to handle `()` as unit value, not just closure

---

## Issue 2: epoll.zen - Unit type in generic args

**File:** `stdlib/io/mux/epoll.zen`
**Status:** ✅ FIXED

**Changes:**
- Modified type parser to recognize `()` as unit type (AstType::Void)
- Modified expression parser to allow `()` as unit value expression

---

## Issue 3: ffi.zen - False "immutable variable" error

**File:** `stdlib/ffi.zen`
**Status:** ✅ FIXED (was cascading from other parser issues)

---

## Issue 4: socket.zen - Negative literal patterns

**File:** `stdlib/io/net/socket.zen`
**Status:** ✅ FIXED

**Changes:**
- Added support for negative number patterns (`-9 => ...`) in pattern parser
- Added block-style pattern matching syntax: `? { pattern => expr, ... }`
- Fixed expression parser for match arm bodies to not greedily consume `-` as subtraction
- Fixed parenthesized expression parsing to recognize `as` for type casts

---

## All Issues Resolved

### Issue 5: socket.zen - Hexadecimal literals not supported

**File:** `stdlib/io/net/socket.zen`
**Line:** 143
**Error:** `Syntax Error: Expected ',' or '}' in struct literal`

**Code:**
```zen
return Ipv4Addr { addr: 0xFFFFFFFF }
```

**Analysis:**
The lexer doesn't support hexadecimal literal syntax (`0x...`).

**Status:** ✅ FIXED - Added hex (0x), binary (0b), and octal (0o) literal support to lexer

---

## Summary of Fixes Made

1. **Bitwise operators** - Full support for `|`, `&`, `^`, `<<`, `>>` in expressions
2. **Address-of operator** - `&expr` now works as prefix unary operator
3. **Unit type `()`** - Works in both type and expression positions
4. **Negative patterns** - `-9 => ...` now works in pattern matching
5. **Block pattern match** - `? { pattern => expr }` syntax now supported
6. **Type cast in parens** - `(a as u32)` now correctly parsed as expression
7. **Hex/binary/octal literals** - `0xFF`, `0b1010`, `0o755` now supported with underscore separators

All tests pass.
