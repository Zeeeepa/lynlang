# Zen Language - Quick Start Guide

## Building

```bash
# Build the compiler
cargo build --release

# Verify it works
./target/release/zen --help
```

## Running Your First Program

Create `hello.zen`:

```zen
{ io } = @std

main = () i32 {
    io.println("Hello, Zen!")
    return 0
}
```

Run it:

```bash
./target/release/zen hello.zen
```

Or compile to executable:

```bash
./target/release/zen hello.zen -o hello
./hello
```

---

## Core Concepts

### 1. Variables - No `let`, No `const`

```zen
x = 10              // Immutable (cannot change)
y ::= 20            // Mutable (can change)

y = y + 1           // OK: y is mutable
// x = x + 1        // ERROR: x is immutable
```

With explicit types:

```zen
x: i32 = 10         // Immutable with type
y:: i32 = 20        // Mutable with type
```

### 2. Pattern Matching - No `if/else`

Zen uses `?` for all conditionals:

```zen
// Single condition
is_ready ? { start() }

// Multiple branches
status ?
    | .Active { process() }
    | .Inactive { wait() }
    | .Error { handle_error() }

// With Option
maybe_value ?
    | Some(v) { io.println("Got: ${v}") }
    | None { io.println("Nothing") }
```

### 3. Functions

```zen
// Function definition
add = (a: i32, b: i32) i32 {
    return a + b
}

// Call traditionally
result = add(5, 3)

// Or use UFC (Uniform Function Call)
result = 5.add(3)
```

### 4. Structs and Enums

```zen
// Struct
Point: {
    x: f64,
    y: f64
}

// Create instance
p = Point { x: 1.0, y: 2.0 }

// Enum
Color: Red, Green, Blue

// Generic enum
Option<T>: Some: T, None
```

### 5. Loops

```zen
// Range loop
(0..10).loop((i) {
    io.println("${i}")
})

// Collection loop
items.loop((item) {
    process(item)
})

// Infinite loop with break
loop(() {
    done ? { break }
})
```

### 6. Error Handling

```zen
// Functions return Result<T, Error>
load_file = (path: string) Result<string, Error> {
    file = File.open(path).raise()  // .raise() returns early on Err
    return Ok(file.read_all())
}

// Handle results with pattern matching
load_file("data.txt") ?
    | Ok(content) { process(content) }
    | Err(e) { io.println("Error: ${e}") }
```

### 7. Memory Management

Zen uses explicit allocators (like Zig):

```zen
{ GPA } = @std.memory.gpa
{ Vec } = @std.collections.vec

// Create allocator
allocator = GPA.new()

// Use it for collections
numbers = Vec<i32>.new(allocator)
numbers.mut_ref().push(1)
numbers.mut_ref().push(2)
numbers.mut_ref().push(3)

// Clean up
numbers.mut_ref().free()
```

---

## Example: Complete Program

```zen
{ io } = @std
{ GPA } = @std.memory.gpa
{ Vec } = @std.collections.vec

sum_vec = (v: Vec<i32>) i32 {
    total ::= 0
    i ::= 0

    loop(() {
        i >= v.len() ? { break }

        v.get(i) ?
            | Some(n) { total = total + n }
            | None {}

        i = i + 1
    })

    return total
}

main = () i32 {
    allocator = GPA.new()

    numbers = Vec<i32>.new(allocator)
    numbers.mut_ref().push(10)
    numbers.mut_ref().push(20)
    numbers.mut_ref().push(30)

    total = sum_vec(numbers)
    io.println("Sum: ${total}")

    numbers.mut_ref().free()
    return 0
}
```

---

## Types Reference

| Category | Types |
|----------|-------|
| Integers | `i8`, `i16`, `i32`, `i64` |
| Unsigned | `u8`, `u16`, `u32`, `u64`, `usize` |
| Floats | `f32`, `f64` |
| Other | `bool`, `void`, `string` |

Core generics:
- `Option<T>` - Some(T) or None
- `Result<T, E>` - Ok(T) or Err(E)
- `Vec<T>` - Growable array
- `Ptr<T>` - Immutable pointer
- `MutPtr<T>` - Mutable pointer

---

## Common Commands

```bash
# Build
cargo build --release

# Run tests
cargo test --all

# Run a .zen file
./target/release/zen file.zen

# Compile to executable
./target/release/zen file.zen -o output

# Start REPL
./target/release/zen
```

---

## Next Steps

- [OVERVIEW.md](OVERVIEW.md) - Complete language documentation
- [INTRINSICS_REFERENCE.md](INTRINSICS_REFERENCE.md) - Low-level primitives
- [examples/](../examples/) - More example programs
