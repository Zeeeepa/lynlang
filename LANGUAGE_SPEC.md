<!-- MESSAGE TO AI CODING AGENTS YOU ARE NOT ALLOWED TO TOUCH THIS THIS IS FOR HUMAN EDITING ONLY -->
# Zenlang Language Specification

**Version**: 1.1.0  
**Status**: Authoritative Reference  
**Last Updated**: 2025-01-20

This document serves as the definitive language specification for Zenlang. All implementations, tools, and code must adhere to these specifications.

## Table of Contents

1. [Core Philosophy](#core-philosophy)
2. [Lexical Structure](#lexical-structure)
3. [Type System](#type-system)
4. [Variable Declarations](#variable-declarations)
5. [Functions](#functions)
6. [Control Flow](#control-flow)
7. [Data Structures](#data-structures)
8. [Behaviors](#behaviors)
9. [Async and Concurrency](#async-and-concurrency)
10. [Module System](#module-system)
11. [Memory Management](#memory-management)
12. [Error Handling](#error-handling)
13. [Foreign Function Interface](#foreign-function-interface)
14. [Testing](testing)
15. [Build System](#build-system)
16. [Metaprogramming](#metaprogramming)
17. [Standard Library](#standard-library)
18. [Syntax Quick Reference](#syntax-quick-reference)

---

## Core Philosophy

### Design Principles
1. **Clarity over cleverness** - Code readability is paramount
2. **Explicit over implicit** - No hidden control flow or allocations
3. **Minimal but composable** - Small set of powerful primitives
4. **Errors as values** - No exceptions, use Result/Option types
5. **Zero-cost abstractions** - Performance without compromise
6. **Colorless async** - No function coloring with async/await

### Non-Negotiable Rules
- **NO** `if`/`else/match` keywords - Use `?` operator exclusively
- **NO** exceptions - All errors are values
- **NO** null pointers - Use Option<T> for optional values
- **NO** implicit conversions - All type conversions must be explicit
- **NO** undefined behavior in safe code
- **NO** lifetime annotations - Smart pointers handle safety
- **NO** raw `&` or `*` - Use Ptr<T> and .value/.address
- **NO** tuples - Use structs for all product types

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

/// Documentation comment
/// Used for API documentation
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
- `@` - Compiler directives prefix
- `#` - Attribute prefix

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
// Arrays - Fixed size, consistent syntax
[N, T]            // Array of N elements of type T
[5, i32]          // Array of 5 i32s
[1024, u8]        // Byte buffer

// Slices - Dynamic views
Slice<T>          // Slice type (dynamic view into array)

// Pointers - No raw & or * symbols
RawPtr<T>         // Raw pointer for FFI only
Ptr<T>            // Smart pointer with ownership
Ref<T>            // Reference-counted pointer

// Access via properties
ptr.value         // Dereference pointer
ptr.address       // Get memory address

// Function types
(Args) ReturnType // Function signature
```

### Type Aliases
```zen
type UserId: u64
type Point2D: { x: f64, y: f64 }
type Handler: (Request) Response
```

### Generic Types
```zen
// Generic function
identity<T> = (value: T) T { value }

// Generic struct
Vec<T> = {
    data: Ptr<[?, T]>,
    len: usize,
    capacity: usize,
}

// Multiple type parameters
HashMap<K, V> = {
    buckets: Ptr<[?, Bucket<K, V>]>,
    count: usize,
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
buffer:: [1024, u8]

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
name: (parameters) ReturnType = {
    // body
}
```

### Parameter Forms
```zen
// No parameters
greet: () void = { }

// Single parameter
double: (x: i32) i32 = { x * 2 }

// Multiple parameters
add: (a: i32, b: i32) i32 = { a + b }

// Default parameters
format: (value: i32, base: u8 = 10) string = { }

// Generic parameters
swap<T>: (a: Ptr<T>, b: Ptr<T>) void = { }

// Allocator parameter (for colorless async)
read: (path: string, alloc: Ptr<Allocator>) Slice<u8> = { }
```

### Return Rules
```zen
// Explicit return
factorial: (n: u64) u64 = {
    n <= 1 ? 
        | true => return 1
        | false => return n * factorial(n - 1)
}

// Implicit return (last expression)
square: (x: i32) i32 = {
    x * x  // No semicolon, implicitly returned
}
```

### UFCS (Uniform Function Call Syntax)
```zen
// Define function with receiver as first param
area: (rect: Rectangle) f64 = {
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

// Struct destructuring (NO TUPLES)
point ? | { x -> x_val, y -> y_val } => "Point($(x_val), $(y_val))"
```

**4. Guards with `->`**
```zen
value ? | v -> v > 100 => "Large"
        | v -> v > 50 => "Medium"
        | v -> v > 0 => "Small"
        | _ => "Zero or negative"
```

**5. Multiple Patterns (or-patterns)**
```zen
day ? | 1 | 2 | 3 | 4 | 5 => "Weekday"
      | 6 | 7 => "Weekend"
      | _ => "Invalid day"
```

**6. Type Patterns**
```zen
value ? | i32 -> n => "Integer: $(n)"
        | string -> s => "String: $(s)"
        | Point -> p => "Point at $(p.x), $(p.y)"
        | _ => "Unknown type"
```

**7. Bool Patters**
```zen

func_that_returns_bool ? { Do this }

// or
func_that_returns_bool ? 
    | 1 => { Do this }

// or 

func_that_returns_bool ? 
    | 1 => { Do this }
    | 0 => { Do that }

func_that_returns_bool ? 
    | true => { Do this }


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

#### Range Iteration
```zen
range(0,10) // or 0..10

// Iterate over range
(0..10).loop((i) => {
    print("Index: $(i)")
})

// With step
(0..100).step(2).loop((i) => {
    print("Even: $(i)")
})
```

#### Collection Iteration
```zen
// Iterate over collections
items.loop((item) => {
    process(item)
})

// With index
items.enumerate().loop((index, value) => {
    print("$(index): $(value)")
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
Person: {
    name: string,
    age: u32,
    email:: Option<string> = None,  // Mutable with default
}

// Generic struct
Point<T>: {
    x: T,
    y: T,
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

// Generic instantiation
origin := Point<f64>{ x: 0.0, y: 0.0 }
```

#### Field Access
```zen
// Read
name := person.name

// Write (only mutable fields)
person.email = Some("new@example.com")

// Method-like access via UFCS
distance := origin.calculate_distance(target)
```

### Enums (Sum Types)

#### Definition
```zen
Result<T, E>: Ok(value: T) | Err(error: E)

Message: Text(content: string) 
    | Image({ url: string, width: u32, height: u32 })
    | Video(url: string)
    | Empty
```

#### Construction
```zen
success := Result::Ok(42)
failure := Result::Err("Not found")
msg := Message::Image({ url: "pic.jpg", width: 800, height: 600 })
empty := Message::Empty
```

#### Pattern Matching
```zen
result ? | .Ok -> val => print("Success: $(val)")
         | .Err -> err => print("Error: $(err)")

message ? | .Text -> content => display_text(content)
          | .Image -> img => render_image(img.url, img.width, img.height)
          | .Video -> url => play_video(url)
          | .Empty => show_placeholder()
```

---

## Behaviors

Instead of traditional traits/interfaces, Zen uses behaviors - structural contracts that types can satisfy.

### Defining Behaviors

```zen
// Behaviors are structs containing function pointers
Comparable<T>: {
    compare: (a: T, b: T) i32,
}

Hashable<T>: {
    hash: (value: T) u64,
}

Serializable<T>: {
    serialize: (value: T, writer: Ptr<Writer>) Result<void, Error>,
    deserialize: (reader: Ptr<Reader>) Result<T, Error>,
}
```

### Providing Implementations

```zen
// Create behavior instances for types
i32_comparable := Comparable<i32>{
    compare: (a: i32, b: i32) i32 {
        a < b ? | true => -1
                | false => a > b ?  | true => 1
                                    | false => 0
    }
}

person_hashable := Hashable<Person>{
    hash: (p: Person) u64 {
        hash_combine(hash_string(p.name), hash_u32(p.age))
    }
}
```

### Using Behaviors in Generics

```zen
// Functions accept behavior instances
sort<T> = (items: Ptr<Slice<T>>, cmp: Comparable<T>) void {
    // Use cmp.compare for sorting
    (0..items.value.len()).loop((i) => {
        ((i+1)..items.value.len()).loop((j) => {
            cmp.compare(items.value[i], items.value[j]) > 0 ?
                | true => swap(items, i, j)
                | false => {}
        })
    })
}

// Usage
numbers := [5, i32]{ 3, 1, 4, 1, 5 }
sort(Ptr::new(numbers.to_slice()), i32_comparable)
```

### Automatic Derivation

```zen
// Compiler can auto-generate common behaviors
#derive(Comparable, Hashable)
Point: {
    x: f64,
    y: f64,
}

// Use auto-generated behaviors
p1 := Point{ x: 1.0, y: 2.0 }
p2 := Point{ x: 3.0, y: 4.0 }
result := Point_comparable.compare(p1, p2)
```

---

## Async and Concurrency

### Colorless Async via Allocators

Zen uses allocator-based colorless async - functions aren't marked async/await. The allocator parameter determines execution mode.

```zen
// Allocator trait includes execution context
Allocator: {
    // Memory operations
    alloc: <T>(size: usize) Ptr<T>,
    free: <T>(ptr: Ptr<T>) void,
    
    // Execution mode
    is_async: bool,
    
    // Async operations (no-op for sync)
    suspend: () Option<Continuation>,
    resume: (cont: Continuation) void,
}

// Same function works sync or async
read_file = (path: string, alloc: Ptr<Allocator>) Result<Slice<u8>, Error> {
    fd := fs.open(path) ? | .Ok -> f => f
                          | .Err -> e => return .Err(e)
    defer fs.close(fd)
    
    buffer := alloc.value.alloc([4096, u8])
    defer alloc.value.free(buffer)
    
    // Allocator handles execution mode internally
    bytes_read := alloc.value.is_async ?
        | true => {
            // Async path - may suspend/resume
            cont := alloc.value.suspend()
            io.read_async(fd, buffer, cont)
        }
        | false => {
            // Sync path - blocks
            io.read_sync(fd, buffer)
        }
    
    .Ok(buffer.slice(0, bytes_read))
}

// Usage - same code, different execution
main = () void {
    // Synchronous execution
    sync_alloc := Ptr::new(SyncAllocator::new())
    data := read_file("config.zen", sync_alloc)
    
    // Asynchronous execution
    async_alloc := Ptr::new(AsyncAllocator::with_runtime(Runtime::init()))
    data := read_file("config.zen", async_alloc)
}
```

### Channels and Message Passing

```zen
Channel<T> = {
    send: (msg: T) void,
    receive: () T,
    try_receive: () Option<T>,
    close: () void,
}

// Actor pattern
Actor<State, Msg> = {
    state:: State,
    mailbox: Channel<Msg>,
    
    run: (self: Ptr<Actor<State, Msg>>) void {
        loop {
            msg := self.value.mailbox.receive()
            self.value.handle_message(msg)
        }
    },
    
    handle_message: (self: Ptr<Actor<State, Msg>>, msg: Msg) void,
}
```

### Threads and Atomics

```zen
// Spawn threads
Thread: {
    spawn: <T>(func: () T) ThreadHandle<T>,
    current: () ThreadId,
    yield_now: () void,
    sleep: (duration: Duration) void,
}

// Atomic operations
Atomic<T> = {
    new: (value: T) Atomic<T>,
    load: (ordering: Ordering) T,
    store: (value: T, ordering: Ordering) void,
    compare_exchange: (current: T, new: T, ordering: Ordering) Result<T, T>,
    fetch_add: (val: T, ordering: Ordering) T,
}

// Usage
counter := Atomic<u64>::new(0)
old := counter.fetch_add(1, .SeqCst)
value := counter.load(.Acquire)
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

// Import standard modules
io := build.import("io")
mem := build.import("mem")
collections := build.import("collections")
thread := build.import("thread")
fs := build.import("fs")
```

### Module Organization

Each `.zen` file is a module. Public items are exported by default unless marked private.

```zen

// Public by default
PI := 3.14159

// Public function
sin = (angle: f64) f64 { /* implementation */ }

// Private function (not exported)
helper = (x: f64) f64 { /* internal use only */ }

// Nested namespace
Trig: {
    sin: sin,
    cos: (angle: f64) f64 { /* implementation */ },
    tan: (angle: f64) f64 { sin(angle) / cos(angle) },
}
```

### Import System

```zen
// Import entire module
math := @std.build.import("math")
value := math.sin(math.PI / 2)

// Import specific items
{ sin, cos, PI } := @std.build.import("math")
value := sin(PI / 2)

// Import with alias
m := @std.build.import("math")
```

---

## Memory Management

### Pointer Types

| Type | Safety | Ownership | Use Case |
|------|--------|-----------|----------|
| `RawPtr<T>` | Unsafe | None | FFI only |
| `Ptr<T>` | Safe | Single owner | General use |
| `Ref<T>` | Safe | Reference counted | Shared ownership |

### Pointer Operations

```zen
// Creating pointers
ptr := Ptr::new(42)           // Heap allocate
ptr := Ptr::from(stack_value) // Move to heap

// Accessing values
value := ptr.value    // Dereference
addr := ptr.address   // Get memory address

// Reference counting
ref1 := Ref::new(data)
ref2 := ref1.clone()  // Increases ref count
// Both refs dropped, data freed

// Raw pointers (unsafe, FFI only)
raw := RawPtr::from_address(0x1234)
value := raw.value  // May crash if invalid
```

### Allocation Strategy

```zen
// Stack allocation (default)
value := 42
array := [100, i32]

// Heap allocation (explicit)
heap_value := Ptr::new(42)
heap_array := Ptr::new([100, i32])

// Custom allocator
alloc := mem.GPA::new()
ptr := alloc.alloc<i32>()
defer alloc.free(ptr)
```

### GPA (General Purpose Allocator)

```zen
GPA: {
    // Core allocation
    alloc: <T>() Ptr<T>,
    alloc_array: <T>(count: usize) Ptr<[?, T]>,
    free: <T>(ptr: Ptr<T>) void,
    realloc: <T>(ptr: Ptr<T>, new_size: usize) Ptr<T>,
    
    // Memory management
    reset: () void,
    destroy: () void,
    
    // Statistics
    bytes_allocated: () usize,
    peak_bytes: () usize,
    allocation_count: () usize,
}

// Usage
gpa := mem.GPA::new()
defer gpa.destroy()

vec := Vec<i32>::with_allocator(Ptr::new(gpa))
vec.push(1, 2, 3)

io.print("Allocated: $(gpa.bytes_allocated()) bytes")
```

### Ownership Rules

1. Each value has single owner
2. Assignment moves ownership (unless type is Copy)
3. Ptr<T> automatically freed when owner scope ends
4. No dangling pointers in safe code
5. No lifetime annotations needed

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
parse_config = (path: string) Result<Config, Error> {
    // Manual handling
    file_result := read_file(path)
    content := file_result ? | .Ok -> data => data
                            | .Err -> err => return .Err(err)
    
    // Try operator (sugar for above)
    json_result := parse_json(content)
    config := json_result ? | .Ok -> c => c
                           | .Err -> e => return .Err(e)
    
    .Ok(config)
}

// Error types
FileError =
    | NotFound(path: string)
    | PermissionDenied(path: string)
    | IoError(message: string)
```

### Panic and Recovery

```zen
// Panic for unrecoverable errors
invariant_violated := true
invariant_violated ? | true => panic("Invariant violation!")
                     | false => {}

// Defer for cleanup
file := open_file("data.txt")
defer {
    file.close()  // Runs even if panic occurs
}
```

---

## Foreign Function Interface

### C Interop with Builder Pattern

```zen
// FFI builder for safe C interop
FFI: {
    lib: (name: string) LibBuilder,
}

LibBuilder: {
    path: (p: string) LibBuilder,
    function: (name: string, sig: FnSignature) LibBuilder,
    constant: (name: string, type: Type) LibBuilder,
    build: () Library,
}

// Define C bindings
sqlite := FFI.lib("sqlite3")
    .path("/usr/lib/libsqlite3.so")
    .function("sqlite3_open", { 
        params: [string, Ptr<RawPtr<void>>],
        returns: i32,
    })
    .function("sqlite3_close", {
        params: [RawPtr<void>],
        returns: i32,
    })
    .constant("SQLITE_OK", i32)
    .build()

// Use C functions safely
db_ptr:: RawPtr<void>
result := sqlite.sqlite3_open("database.db", Ptr::new(db_ptr))

result == sqlite.SQLITE_OK ? 
    | true => print("Database opened")
    | false => print("Failed to open database")

defer sqlite.sqlite3_close(db_ptr)
```

### Platform-Specific Code

```zen
// Conditional compilation based on target
get_home: () string = {
    @std.target.os ?
        | .linux => @env("HOME") ?? "/home/user"
        | .windows => @env("USERPROFILE") ?? "C:\\Users\\User"  
        | .macos => @env("HOME") ?? "/Users/user"
        | .wasm => "/"  // No filesystem in WASM
        | _ => @compile_error("Unsupported platform")
}

// Or use compile-time conditions
comptime {
    @std.target.os == .windows ?
        | true => {
            // Windows-specific type definitions
            Handle := Ptr<void>
        }
        | false => {
            // Unix-like systems
            Handle := i32
        }
}

// Architecture-specific optimizations
fast_multiply: (a: u64, b: u64) u128 = {
    @std.target.arch ?
        | .x86_64 => {
            // Use x86-64 specific instructions
            @intrinsic("mul_u128", a, b)
        }
        | .aarch64 => {
            // Use ARM64 specific instructions
            @intrinsic("umulh", a, b)
        }
        | _ => {
            // Fallback implementation
            (a as u128) * (b as u128)
        }
}
```

---

## Testing

### Test Framework

```zen
test, assert, expect_panic = @std.testing
// Test blocks
test("array operations") {
    arr : [5, i32]
    arr = [ 1, 2, 3, 4, 5 ]

    assert(arr.len() == 5)
    assert(arr[0] == 1)
    assert(arr[4] == 5)
}

test("async operations") {
    // Tests use sync allocator for determinism
    alloc := TestAllocator::new()
    defer assert(alloc.bytes_leaked() == 0)
    
    result := read_file("test.txt", Ptr::new(alloc))
    result ? | .Ok -> data => assert(data.len() > 0)
             | .Err -> _ => assert(false, "File should exist")
}

// Test utilities
assert(condition: bool, message: string = "")
assert_eq<T>(left: T, right: T)
assert_ne<T>(left: T, right: T)
expect_panic(func: () void)
```

### Running Tests

```bash
zen test              # Run all tests
zen test pattern     # Run tests matching pattern
zen test --verbose   # Show detailed output
zen test --coverage  # Generate coverage report
```

---

## Build System

### Build Configuration (build.zen)

```zen
Build := @std.Build
enum_to_string := @std.utils

build = (b: Ptr<Build>) void {
    // Project metadata
    b.value.name = "my-app"
    b.value.version = "1.0.0"
    
    // Build options
    target := b.value.standard_target_options()
    optimize := b.value.standard_optimize_option()
    
    // Execution mode option
    mode := b.value.option(enum{sync, async, auto}, "mode", "Execution mode") ?? .auto
    
    // Main executable
    exe := b.value.add_executable({
        name: "my-app",
        root: "src/main.zen",
        target: target,
        optimize: optimize,
    })
    
    // Pass compile-time configuration
    exe.value.define_symbol("EXECUTION_MODE", enum_to_string(mode))
    exe.value.define_symbol("VERSION", b.value.version)
    
    // Add dependencies
    exe.value.add_package("http", "libs/http/mod.zen")
    exe.value.link_library("sqlite3")
    
    // Install
    b.value.install_artifact(exe)
    
    // Run step
    run_cmd := exe.value.run()
    run_step := b.value.step("run", "Run the application")
    run_step.value.depend_on(run_cmd)
    
    // Test step
    tests := b.value.add_test("src/main.zen")
    test_step := b.value.step("test", "Run tests")
    test_step.value.depend_on(tests)
}
```

### CLI Commands

```bash
# Build and run
zen run main.zen          # Direct run single file
zen build                 # Build project
zen build --release      # Release build
zen run                  # Build and run project

# Development
zen test                 # Run tests
zen fmt                  # Format code
zen check               # Type check
zen clean              # Clean build artifacts

# Cross-compilation
zen build --target=wasm
zen build --target=windows-x64
```

---

## Metaprogramming

### Compile-Time Execution

```zen
// Comptime blocks execute at compile time
TABLE := comptime {
    table:: [256, u8]
    loop (0..256, (i) =>{
        table[i] = compute_crc_byte(i)
    })
    table
}

// Compile-time parameters
make_array<comptime N: usize, T> = () [N, T] {
    result:: [N, T]
    loop (0..N, (i){
        result[i] = T::default()
    })
    result
}
```

### Type Reflection

```zen
// Type introspection
size := size_of(MyStruct)
align := align_of(MyStruct)
type_name := type_name(MyStruct)

// Field reflection
fields := fields(Person)
fields.each(field=>{
    print("Field: $(field.name), Type: $(field.type)")
})

// Function reflection
info := fn_info(my_function)
param_count := info.params.len()
return_type := info.return_type
```

### Code Generation

```zen
// Generate code at compile time

generate_enum(name:str, values: Vec<str>){
    Ast.Enum(name, values)
}
generate_enum("Status", ["Active", "Inactive", "Pending"])



// Generates:
// Status = 
//     | Active
//     | Inactive
//     | Pending

// Usage
result := assert_positive!(compute_value())
```

---

## Standard Library

### Core Modules

| Module | Purpose | Key Types/Functions |
|--------|---------|-------------------|
| `io` | Input/Output | `print`, `read_line`, `File`, `stdin`, `stdout` |
| `mem` | Memory | `Allocator`, `GPA`, `Arena`, `Pool` |
| `collections` | Data structures | `Vec<T>`, `HashMap<K,V>`, `List<T>`, `Set<T>` |
| `math` | Mathematics | `sin`, `cos`, `sqrt`, `pow`, `min`, `max` |
| `string` | String utilities | `split`, `join`, `format`, `parse` |
| `fs` | File system | `read_file`, `write_file`, `exists`, `mkdir` |
| `thread` | Concurrency | `Thread`, `Mutex`, `Channel`, `Atomic` |
| `net` | Networking | `TcpSocket`, `UdpSocket`, `Http` |
| `time` | Time/Date | `Instant`, `Duration`, `DateTime` |
| `crypto` | Cryptography | `hash`, `encrypt`, `random` |

### Import Examples

```zen
// Import standard modules
build := @std.build
io := build.import("io")
mem := build.import("mem")
{ Vec, HashMap } := build.import("collections")

// Use imported functionality
main: () void = {
    gpa := mem.GPA::new()
    defer gpa.destroy()
    
    vec := Vec<string>::with_allocator(Ptr::new(gpa))
    vec.push("Hello")
    vec.push("World")
    
    loop item in vec.iter() {
        io.print(item)
    }
}
```

---

## Syntax Quick Reference

### Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+` `-` `*` `/` `%` `**` (power) |
| Comparison | `==` `!=` `<` `>` `<=` `>=` |
| Logical | `&&` `||` `!` |
| Bitwise | `&` `|` `^` `<<` `>>` `~` |
| Range | `..` (exclusive) `..=` (inclusive) |
| Pattern | `?` (match) `->` (bind) `=>` (result) |
| Assignment | `=` `:=` `::=` |
| Member | `.` (access) `::` (namespace) |
| Optional | `??` (unwrap or default) |

### Precedence (High to Low)

1. Postfix: `.` `::`  `()` `[]`
2. Prefix: `!` `~` `-` (unary)
3. Power: `**`
4. Multiplicative: `*` `/` `%`
5. Additive: `+` `-`
6. Shift: `<<` `>>`
7. Bitwise AND: `&`
8. Bitwise XOR: `^`
9. Bitwise OR: `|`
10. Comparison: `<` `>` `<=` `>=` `==` `!=`
11. Logical AND: `&&`
12. Logical OR: `||`
13. Range: `..` `..=`
14. Null coalesce: `??`
15. Pattern match: `?`
16. Assignment: `=` `:=` `::=`

---

## Code Style Guidelines

### Naming Conventions

- **Types**: `PascalCase` (e.g., `HttpRequest`)
- **Functions/Variables**: `snake_case` (e.g., `parse_input`)
- **Constants**: `UPPER_SNAKE_CASE` (e.g., `MAX_SIZE`)
- **Generic Parameters**: Single letter or `PascalCase` (e.g., `T`, `Element`)
- **Modules**: `lowercase` (e.g., `math`, `collections`)

### Formatting Rules

- **Indentation**: 4 spaces (no tabs)
- **Line Length**: 100 characters preferred, 120 max
- **Braces**: Same line for definitions
- **Trailing Commas**: Required in multi-line
- **Blank Lines**: One between top-level items

### Example Style

```zen
// Good Zen style
HttpServer: {
    port: u16,
    handler:: Option<Handler>,
    max_connections: usize = 1000,
}

create_server = (port: u16, alloc: Ptr<Allocator>) Result<HttpServer, Error> {
    port > 0 ? 
        | true => {
            server := HttpServer{ 
                port: port,
                handler: None,
                max_connections: 1000,
            }
            .Ok(server)
        }
        | false => .Err(Error::InvalidPort)
}

// Pattern matching alignment
handle_request = (req: Request) Response {
    req.method ? 
        | .Get -> path    => handle_get(path)
        | .Post -> body   => handle_post(body)
        | .Delete -> id   => handle_delete(id)
        | _               => Response::method_not_allowed()
}
```

---

## Version History

- **1.1.0** (2025-01-20): Added async system, behaviors, FFI, testing, comprehensive updates
- **1.0.0** (2025-09-04): Initial comprehensive specification

---

## Compliance Notes

This specification is the authoritative reference. Any deviations in implementation should be considered bugs. When in doubt, this document supersedes all other documentation.

**Core tenets**: 
- No `if`/`else` - only `?` operator
- No exceptions - errors are values
- No lifetime annotations - smart pointers
- No raw `&`/`*` - use Ptr<T>
- No tuples - use structs
- No function coloring - allocators determine async

Keep it Zen.