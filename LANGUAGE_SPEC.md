# Zenlang Language Specification

**Version**: 1.0.0  
**Status**: Authoritative Reference  
**Last Updated**: 2025-09-04

This document serves as the definitive language specification for Zenlang. All implementations, tools, and code must adhere to these specifications.

## Table of Contents

1. [Core Philosophy](#core-philosophy)
2. [Lexical Structure](#lexical-structure)
3. [Type System](#type-system)
4. [Variable Declarations](#variable-declarations)
5. [Functions](#functions)
6. [Control Flow](#control-flow)
7. [Data Structures](#data-structures)
8. [Module System](#module-system)
9. [Memory Management](#memory-management)
10. [Error Handling](#error-handling)
11. [Metaprogramming](#metaprogramming)
12. [Standard Library](#standard-library)
13. [Syntax Quick Reference](#syntax-quick-reference)

---

## Core Philosophy

### Design Principles
1. **Clarity over cleverness** - Code readability is paramount
2. **Explicit over implicit** - No hidden control flow or allocations
3. **Minimal but composable** - Small set of powerful primitives
4. **Errors as values** - No exceptions, use Result/Option types
5. **Zero-cost abstractions** - Performance without compromise

### Non-Negotiable Rules
- **NO** `if`/`else` keywords - Use `?` operator exclusively
- **NO** exceptions - All errors are values
- **NO** null pointers - Use Option<T> for optional values
- **NO** implicit conversions - All type conversions must be explicit
- **NO** undefined behavior in safe code

---

## Lexical Structure

### Source Files
- **Extension**: `.zen`
- **Encoding**: UTF-8 (required)
- **Line Endings**: LF or CRLF (normalized to LF)
- **Entry Point**: `main = () ReturnType { }`

### Comments
```zen
// Single-line comment

/* 
   Multi-line comment
   Can span multiple lines
*/
```

### Identifiers
- Start with letter or underscore
- Contain letters, digits, underscores
- Case-sensitive
- No keywords (language is keyword-minimal)

### Reserved Symbols
- `?` - Pattern matching operator
- `->` - Destructuring/binding in patterns
- `=>` - Pattern result separator
- `:=` - Immutable binding
- `::=` - Mutable binding
- `=` - Assignment/definition
- `@` - Compiler namespace prefix

---

## Type System

### Primitive Types

| Type | Description | Default Value |
|------|-------------|---------------|
| `bool` | Boolean | `false` |
| `void` | No value | N/A |
| `i8`, `i16`, `i32`, `i64` | Signed integers | `0` |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers | `0` |
| `usize` | Pointer-sized unsigned | `0` |
| `f32`, `f64` | Floating point | `0.0` |
| `string` | UTF-8 string | `""` |

### Composite Types

```zen
// Arrays - Fixed size
[N]T              // Array of N elements of type T
[5]i32            // Array of 5 i32s

// Slices - Dynamic view
[]T               // Slice of T

// Pointers
RawPtr<T>         // Raw pointer to T
Ptr<T>            // Smart pointer (unsafe)
Ref<T>            // Managed reference (safe)

// Function types
(Args) ReturnType // Function signature
```

### Type Aliases
```zen
type UserId = u64
type Point2D = { x: f64, y: f64 }
```

### Generic Types
```zen
// Generic function
identity<T> = (value: T) T { value }

// Generic struct
Vec<T> = {
    data: RawPtr<T>,
    len: usize,
    capacity: usize,
}
```

---

## Variable Declarations

### Declaration Syntax Rules

| Syntax | Mutability | Type | Scope | Usage |
|--------|------------|------|-------|-------|
| `name := value` | Immutable | Inferred | Local | Primary immutable binding |
| `name ::= value` | **Mutable** | Inferred | Local | Primary mutable binding |
| `name: T = value` | Immutable | Explicit | Local | Typed immutable |
| `name:: T = value` | **Mutable** | Explicit | Local | Typed mutable |
| `name:: T` | **Mutable** | Explicit | Local | Default initialized |

### Examples
```zen
// Immutable bindings (constants)
PI := 3.14159
MAX_SIZE: u32 = 1000

// Mutable variables
counter ::= 0
buffer  :: [1024, u8]

// Assignment (only to mutable)
counter = counter + 1
```

### Scoping Rules
- Block-scoped with lexical scoping
- Shadowing allowed in nested scopes
- No hoisting - declarations must precede use

---

## Functions

### Function Definition Syntax
```zen
name = (parameters) ReturnType {
    // body
}
```

### Parameter Forms
```zen
// No parameters
greet = () void { }

// Single parameter
double = (x: i32) i32 { x * 2 }

// Multiple parameters
add = (a: i32, b: i32) i32 { a + b }

// Default parameters
format = (value: i32, base: u8 = 10) string { }

// Generic parameters
swap<T> = (a: *T, b: *T) void { }
```

### Return Rules
```zen
// Explicit return
factorial = (n: u64) u64 {
    n <= 1 ? 
        | true => return 1
        | false => return n * factorial(n - 1)
}

// Implicit return (last expression)
square = (x: i32) i32 {
    x * x  // No semicolon, implicitly returned
}
```

### UFCS (Uniform Function Call Syntax)
```zen
// Define function with receiver as first param
area = (rect: Rectangle) f64 {
    rect.width * rect.height
}

// Call as method
my_rect := Rectangle{ width: 10, height: 5 }
result := my_rect.area()  // UFCS call
```

---

## Control Flow

### Pattern Matching (`?` operator)

**Critical**: Zen has NO `if`, `else`, `switch`, or `match` keywords. All conditional logic uses `?`.

#### Basic Syntax
```zen
scrutinee ? | pattern => expression
            | pattern => expression
            | _ => default_expression
```

#### Pattern Types

**1. Value Patterns**
```zen
score ? | 100 => "Perfect"
        | 0 => "Failed"
        | _ => "In progress"
```

**2. Range Patterns**
```zen
age ? | 0..=12 => "Child"
      | 13..=19 => "Teen"
      | 20..=64 => "Adult"
      | _ => "Senior"
```

**3. Destructuring with `->`**
```zen
// Enum destructuring
result ? | .Ok -> value => process(value)
         | .Err -> error => handle_error(error)

// Struct destructuring
point ? | { x -> x_val, y -> y_val } => "Point($(x_val), $(y_val))"
```

**4. Guards with `->`**
```zen
value ? | v -> v > 100 => "Large"
        | v -> v > 50 => "Medium"
        | v -> v > 0 => "Small"
        | _ => "Zero or negative"
```

### Loops

**Only ONE loop keyword: `loop`**

#### Conditional Loop
```zen
// While-like
counter ::= 10
loop (counter > 0) {
    print(counter)
    counter = counter - 1
}
```

#### Infinite Loop
```zen
loop {
    input := get_input()
    input == "quit" ? | true => break
    process(input)
}
```

#### Functional Iteration
```zen
// Range iteration
range(0, 10).loop(i -> {
    print("Index: $(i)")
})

// Collection iteration
items.loop(item -> {
    process(item)
})
```

#### Loop Control
- `break` - Exit loop
- `continue` - Skip to next iteration
- Labels for nested loops: `outer: loop { ... break outer ... }`

---

## Data Structures

### Structs (Product Types)

#### Definition
```zen
Person = {
    name: string,
    age: u32,
    email:: Option<string> = None,  // Mutable with default
}
```

#### Instantiation
```zen
// All fields
alice := Person{ 
    name: "Alice",
    age: 30, 
    email: Some("alice@example.com") 
}

// With defaults
bob := Person{ name: "Bob", age: 25 }  // email defaults to None
```

#### Field Access
```zen
// Read
name := person.name

// Write (only mutable fields)
person.email = Some("new@example.com")
```

### Enums (Sum Types)

#### Definition
```zen
Result<T, E> =
    | Ok(value: T)
    | Err(error: E)

Message =
    | Text(content: string)
    | Image({ url: string, width: u32, height: u32 })
    | Video(url: string)
```

#### Construction
```zen
success := Result::Ok(42)
failure := Result::Err("Not found")
msg := Message::Image({ url: "pic.jpg", width: 800, height: 600 })
```

#### Pattern Matching
```zen
result ? | .Ok -> val => print("Success: $(val)")
         | .Err -> err => print("Error: $(err)")
```

---

## Module System

### The `@std` Namespace

**Only built-in namespace**: `@std`

```zen
// Core compiler intrinsics
core := @std.core

// Build system interface
build := @std.build

// Import modules
io := build.import("io")
mem := build.import("mem")
collections := build.import("collections")
```

### Module Organization
- Each `.zen` file is a module
- Public items are exported by default
- Use nested blocks for sub-namespacing

```zen
Math = {
    PI := 3.14159,
    
    sin = (angle: f64) f64 { /* implementation */ },
    cos = (angle: f64) f64 { /* implementation */ },
}

// Usage
value := Math.sin(Math.PI / 2)
```

---

## Memory Management

### Pointer Types

| Type | Safety | Nullable | Use Case |
|------|--------|----------|----------|
| `RawPtr<T>` | Unsafe | Yes | C FFI, low-level |
| `Ptr<T>` | Unsafe | No (uses Option) | Systems programming |
| `Ref<T>` | Safe | No | General use |

### Allocation Strategy
```zen
// Stack allocation (default)
value := 42

// Heap allocation (explicit)
allocator := mem.get_allocator()
ptr := allocator.alloc<i32>()
defer allocator.free(ptr)
```

### GPA Allocator
The General Purpose Allocator (GPA) provides efficient memory management with configurable strategies.

```zen
// Create GPA allocator
gpa := mem.GPA.new()
defer gpa.destroy()

// Allocate memory
ptr := gpa.alloc<i32>()
defer gpa.free(ptr)

// Allocate array
arr := gpa.alloc_array<i32>(100)
defer gpa.free_array(arr)

// Reallocate memory
new_ptr := gpa.realloc(ptr, 200)
defer gpa.free(new_ptr)
```

#### GPA Interface
```zen
GPA = struct {
    // Core allocation methods
    alloc: <T>() Ptr<T>
    alloc_array: <T>(count: usize) Ptr<T>
    free: <T>(ptr: Ptr<T>) void
    free_array: <T>(ptr: Ptr<T>, count: usize) void
    realloc: <T>(ptr: Ptr<T>, new_size: usize) Ptr<T>
    
    // Memory management
    reset: () void
    destroy: () void
    
    // Statistics
    bytes_allocated: () usize
    peak_bytes: () usize
    allocation_count: () usize
}
```

#### GPA Usage Examples
```zen
// Basic allocation and cleanup
gpa := mem.GPA.new()
defer gpa.destroy()

// Allocate single value
ptr := gpa.alloc<i32>()
defer gpa.free(ptr)
ptr.value = 42

// Allocate array (known size)
arr := gpa.alloc_array<f64>(100)
defer gpa.free_array(arr, 100)

// Use GPA with Vec
vec := Vec<i32>.new_with_allocator(gpa)
vec.push(1, 2, 3)
io.print("Vec size: $(vec.len())")
```

### Ownership Rules
1. Each value has single owner
2. References cannot outlive owner
3. Mutable XOR shared references

---

## Error Handling

### Result Type
```zen
Result<T, E> =
    | Ok(value: T)
    | Err(error: E)
```

### Option Type
```zen
Option<T> =
    | Some(value: T)
    | None
```

### Error Propagation
```zen
parse_config = () Result<Config, Error> {
    // Manual handling
    file_result := read_file("config.zen")
    content := file_result ? | .Ok -> data => data
                            | .Err -> err => return .Err(err)
    
    // Parse and return
    parse(content)
}
```

---

## Metaprogramming

### Compile-Time Execution
```zen
// Comptime block
TABLE := comptime {
    table:: [256, u8]
    i ::= 0
    loop i < 256 {
        table[i] = compute_value(i)
        i = i + 1
    }
    table
}
```

### Generic Functions
```zen
// Type parameters
min<T> = (a: T, b: T) T {
    a < b ? | true => a 
            | false => b
}

// Compile-time values
make_array<comptime N: usize, T> = () [N]T {
    [N, T{}]  // Default initialized
}
```

### Type Reflection
```zen
size := @std.meta.sizeOf(MyStruct)
align := @std.meta.alignOf(MyStruct)
fields := @std.meta.fields(MyStruct) // if is a function fields.$params, fields.body

```

---

## Standard Library

### Core Modules

| Module | Purpose | Key Types/Functions |
|--------|---------|-------------------|
| `io` | Input/Output | `print`, `read_line`, `File` |
| `mem` | Memory | `Allocator`, `alloc`, `free` |
| `collections` | Data structures | `Vec<T>`, `HashMap<K,V>`, `List<T>` |
| `math` | Mathematics | `sin`, `cos`, `sqrt`, `pow` |
| `string` | String utilities | `split`, `join`, `format` |
| `fs` | File system | `read_file`, `write_file`, `exists` |

### Import Pattern
```zen
// Standard import pattern
build := @std.build
io := build.import("io")
mem := build.import("mem")
Vec := build.import("collections").Vec

// Usage with GPA allocator
gpa := mem.GPA.new()
defer gpa.destroy()

vec := Vec<i32>.new_with_allocator(gpa)
vec.push(42)
io.print("Size: $(vec.len())")
io.print("Allocated: $(gpa.bytes_allocated()) bytes")
```

---

## Syntax Quick Reference

### Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+` `-` `*` `/` `%` |
| Comparison | `==` `!=` `<` `>` `<=` `>=` |
| Logical | `&&` `||` `!` |
| Bitwise | `&` `|` `^` `<<` `>>` |
| Range | `..` (exclusive) `..=` (inclusive) |
| Pattern | `?` (match) `->` (bind) `=>` (result) |
| Assignment | `=` `:=` `::=` |

### Precedence (High to Low)
1. Member access `.`, Call `()`
2. Unary `!` `-` `*` `&`
3. Multiplicative `*` `/` `%`
4. Additive `+` `-`
5. Shift `<<` `>>`
6. Bitwise AND `&`
7. Bitwise XOR `^`
8. Bitwise OR `|`
9. Comparison `<` `>` `<=` `>=` `==` `!=`
10. Logical AND `&&`
11. Logical OR `||`
12. Pattern match `?`
13. Assignment `=` `:=` `::=`

---

## Code Style Guidelines

### Naming Conventions
- **Types**: `PascalCase` (e.g., `HttpRequest`)
- **Functions/Variables**: `snake_case` (e.g., `parse_input`)
- **Constants**: `UPPER_SNAKE_CASE` (e.g., `MAX_SIZE`)
- **Generic Parameters**: Single uppercase letter or `PascalCase`

### Formatting Rules
- **Indentation**: 4 spaces (no tabs in source)
- **Line Length**: 100 characters preferred, 120 max
- **Braces**: Same line for definitions
- **Trailing Commas**: Recommended in multi-line

### Example
```zen
// Good style example
HttpServer = {
    port: u16,
    handler:: Option<Handler>,
    max_connections: usize = 1000,
    
}

create_server = (port: u16) Result<HttpServer, Error> {
    port > 0 ? 
        | true => .Ok(HttpServer{ 
            port: port, 
            handler: None,
        })
        | false => .Err(InvalidPort)
}
```

---

## Version History

- **1.0.0** (2025-09-04): Initial comprehensive specification
- Based on lang.md conceptual v1.0 and current implementation

---

## Compliance Notes

This specification is the authoritative reference. Any deviations in implementation should be considered bugs. When in doubt, this document supersedes all other documentation.

**Remember**: No `if`, no `else`, no exceptions. Keep it Zen.