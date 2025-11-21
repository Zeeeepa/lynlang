# Zen Standard Library Implementation

**Last Updated**: 2025-11-21  
**Status**: Implementation in Progress  

---

## Overview

The Zen standard library is organized into modules, each providing specific functionality. All modules are written in Zen itself (not Rust), and they build on LLVM primitives defined in the compiler.

---

## Module Structure

```
stdlib/
├── core/               # Core types and utilities
│   ├── option.zen     # Option<T> enum
│   ├── result.zen     # Result<T, E> enum  
│   ├── ptr.zen        # Ptr<T> safe pointer wrapper
│   └── propagate.zen  # Error propagation helpers
├── memory/            # Memory management
│   ├── allocator.zen  # Allocator trait
│   └── gpa.zen        # General Purpose Allocator (malloc/free wrapper)
├── string.zen         # String type (dynamic growable text)
├── vec.zen            # Vec<T> generic growable array
├── io/
│   └── io.zen         # Print, read functions
├── math/
│   └── math.zen       # Trigonometric, exponential, logarithmic
├── collections/       # Data structures
│   ├── stack.zen      # LIFO stack
│   ├── queue.zen      # FIFO queue
│   ├── set.zen        # Unordered unique elements
│   └── hashmap.zen    # Key-value map
├── time.zen           # Duration, Instant, sleep
├── random.zen         # Pseudo-random number generation
├── error.zen          # Error types and levels
├── testing/
│   └── runner.zen     # Testing utilities and assertions
├── fs/
│   └── fs.zen         # File system operations
├── net/
│   └── net.zen        # TCP/UDP sockets
├── ffi/
│   └── ffi.zen        # C FFI and dynamic loading
└── std.zen            # Entry point that exports all modules
```

---

## Core Modules

### Option<T> - Maybe Value
**File**: `stdlib/core/option.zen`

```zen
Option<T>:
    Some: T,
    None
```

A type-safe way to represent values that may or may not exist. No null/nil pointers.

**Functions**:
- `some(value: T) Option<T>` - Wrap value in Some
- `none() Option<T>` - Create None
- `is_some(opt: Option<T>) bool` - Check if Some
- `is_none(opt: Option<T>) bool` - Check if None
- `map(opt: Option<T>, f: (T) U) Option<U>` - Transform value
- `unwrap_or(opt: Option<T>, default: T) T` - Get value or default
- `unwrap(opt: Option<T>) T` - Get value or panic

**Pattern Matching**:
```zen
maybe_num: Option<i32> = Option.Some(42)

maybe_num ?
    | Some(n) { io.println("Got: ${n}") }
    | None { io.println("No value") }
```

---

### Result<T, E> - Success or Error
**File**: `stdlib/core/result.zen`

```zen
Result<T, E>:
    Ok: T,
    Err: E
```

Type-safe error handling without exceptions.

**Functions**:
- `ok(value: T) Result<T, E>` - Wrap success
- `err(error: E) Result<T, E>` - Wrap error
- `is_ok(res: Result<T, E>) bool` - Check if Ok
- `is_err(res: Result<T, E>) bool` - Check if Err
- `map(res: Result<T, E>, f: (T) U) Result<U, E>` - Transform Ok
- `map_err(res: Result<T, E>, f: (E) F) Result<T, F>` - Transform Err
- `unwrap(res: Result<T, E>) T` - Get Ok or panic
- `unwrap_err(res: Result<T, E>) E` - Get Err or panic

**Pattern Matching**:
```zen
file = File.open("data.txt") ?
    | Ok(f) { f }
    | Err(e) { 
        io.eprintln("Failed: ${e}")
        return
    }
```

---

### Ptr<T> - Type-Safe Pointer
**File**: `stdlib/core/ptr.zen`

```zen
Ptr<T>:
    Some: *u8,
    None
```

Safe wrapper around raw pointers with bounds checking.

**Functions**:
- `ptr_allocate(size: usize) Ptr<T>` - Allocate heap memory
- `ptr_from_addr(addr: *u8) Ptr<T>` - Wrap address
- `ptr_none() Ptr<T>` - Create null pointer
- `ptr_is_some(p: Ptr<T>) bool` - Check if valid
- `ptr_is_none(p: Ptr<T>) bool` - Check if null
- `ptr_at(p: Ptr<T>, index: usize) Ptr<T>` - Get element pointer
- `ptr_offset(p: Ptr<T>, count: i64) Ptr<T>` - Advance pointer
- `ptr_addr(p: Ptr<T>) *u8` - Get raw address
- `ptr_unwrap(p: Ptr<T>) *u8` - Unwrap or get null
- `ptr_free(p: Ptr<T>, size: usize)` - Deallocate
- `ptr_copy(src: Ptr<T>, dst: Ptr<T>, count: usize)` - Copy memory
- `ptr_eq(p1: Ptr<T>, p2: Ptr<T>) bool` - Compare pointers

---

## Memory Management

### Allocator Trait
**File**: `stdlib/memory/allocator.zen`

```zen
Allocator: {
    allocate: (self, size: usize) *u8,
    deallocate: (self, ptr: *u8, size: usize) void,
    reallocate: (self, ptr: *u8, old_size: usize, new_size: usize) *u8
}
```

Interface for memory allocation strategies.

---

### GPA - General Purpose Allocator
**File**: `stdlib/memory/gpa.zen`

Simple wrapper around system `malloc`/`free`/`realloc`.

```zen
GPA: {
    id: i32
}

alloc = gpa_new()
ptr = gpa_allocate(alloc, 1024)
gpa_deallocate(alloc, ptr, 1024)
```

**Functions**:
- `gpa_new() GPA` - Create allocator
- `gpa_allocate(alloc: GPA, size: usize) *u8` - Allocate memory
- `gpa_deallocate(alloc: GPA, ptr: *u8, size: usize)` - Free memory
- `gpa_reallocate(...) *u8` - Resize allocation
- `default_allocator() GPA` - Get default instance

---

## Collections

### Vec<T> - Growable Array
**File**: `stdlib/vec.zen`

```zen
Vec<T>: {
    data: *u8,
    len: usize,
    capacity: usize
}
```

Generic dynamic array that grows automatically.

**Functions**:
- `vec_new() Vec<T>` - Create empty
- `vec_with_capacity(cap: usize) Vec<T>` - Pre-allocate
- `vec_len(v: Vec<T>) usize` - Get length
- `vec_capacity(v: Vec<T>) usize` - Get capacity
- `vec_is_empty(v: Vec<T>) bool` - Check empty
- `vec_push(v: *Vec<T>, elem_size: usize, elem_addr: *u8)` - Append
- `vec_pop(v: *Vec<T>)` - Remove last
- `vec_clear(v: *Vec<T>)` - Clear (keep capacity)
- `vec_free(v: *Vec<T>, elem_size: usize)` - Deallocate
- `vec_reserve(v: *Vec<T>, additional: usize, elem_size: usize)` - Reserve space

**Note**: Vec API takes element size and address because Zen doesn't have template method specialization yet.

---

### Stack<T> - LIFO
**File**: `stdlib/collections/stack.zen`

```zen
stack_new<T>() Stack<T>
stack_push(s: *Stack<T>, elem_size: usize, elem_addr: *u8)
stack_pop(s: *Stack<T>)
stack_peek_addr(s: Stack<T>) *u8
stack_free(s: *Stack<T>, elem_size: usize)
```

---

### Queue<T> - FIFO
**File**: `stdlib/collections/queue.zen`

```zen
queue_new<T>() Queue<T>
queue_enqueue(q: *Queue<T>, elem_size: usize, elem_addr: *u8)
queue_dequeue(q: *Queue<T>)
queue_peek_addr(q: Queue<T>) *u8
queue_free(q: *Queue<T>, elem_size: usize)
```

---

### Set<T> - Unique Elements
**File**: `stdlib/collections/set.zen`

Simple unordered collection with naive O(n) lookup.

```zen
set_new<T>() Set<T>
set_insert(s: *Set<T>, elem_size: usize, elem_addr: *u8)
set_remove(s: *Set<T>, index: usize)
set_contains(s: Set<T>, index: usize) bool
set_free(s: *Set<T>, elem_size: usize)
```

---

### HashMap<K, V> - Key-Value Map
**File**: `stdlib/collections/hashmap.zen`

Hash table with linear probing (simplified).

```zen
hashmap_new<K, V>() HashMap<K, V>
hashmap_insert(m: *HashMap<K, V>, key: K, value: V)
hashmap_get(m: HashMap<K, V>, key: K) Option<V>
hashmap_remove(m: *HashMap<K, V>, key: K) Option<V>
hashmap_free(m: *HashMap<K, V>, entry_size: usize)
```

---

## String

**File**: `stdlib/string.zen`

```zen
String: {
    data: *u8,
    len: usize,
    capacity: usize
}
```

Dynamic growable UTF-8 text.

**Functions**:
- `string_new() String` - Create empty
- `string_from_static(s: *u8, s_len: usize) String` - Copy from static
- `string_len(s: String) usize` - Get length
- `string_capacity(s: String) usize` - Get capacity
- `string_is_empty(s: String) bool` - Check empty
- `string_push(s: *String, byte: u8)` - Append byte
- `string_at(s: String, index: usize) u8` - Get byte
- `string_clear(s: *String)` - Clear (keep capacity)
- `string_reserve(s: *String, additional: usize)` - Reserve
- `string_clone(s: String) String` - Deep copy
- `string_free(s: *String)` - Deallocate

---

## Input/Output

**File**: `stdlib/io/io.zen`

```zen
print(message: String) void
println(message: String) void
eprint(message: String) void
eprintln(message: String) void
read_line() Result<String, String>
read_input(prompt: String) Result<String, String>
```

**Constants**:
- `STDIN = 0`
- `STDOUT = 1`
- `STDERR = 2`

---

## Mathematics

**File**: `stdlib/math/math.zen`

**Constants**:
- `PI = 3.14159265358979323846`
- `E = 2.71828182845904523536`

**Trigonometric**:
- `sin(x: f64) f64`
- `cos(x: f64) f64`
- `tan(x: f64) f64`
- `asin(x: f64) f64`
- `acos(x: f64) f64`
- `atan(x: f64) f64`
- `atan2(y: f64, x: f64) f64`

**Hyperbolic**:
- `sinh(x: f64) f64`
- `cosh(x: f64) f64`
- `tanh(x: f64) f64`

**Exponential/Logarithmic**:
- `exp(x: f64) f64` - e^x
- `log(x: f64) f64` - Natural log
- `log10(x: f64) f64` - Base 10 log
- `log2(x: f64) f64` - Base 2 log

**Power**:
- `pow(x: f64, y: f64) f64` - x^y
- `sqrt(x: f64) f64` - Square root
- `cbrt(x: f64) f64` - Cube root

**Rounding**:
- `floor(x: f64) f64`
- `ceil(x: f64) f64`
- `round(x: f64) f64`
- `trunc(x: f64) f64`
- `fabs(x: f64) f64` - Absolute value

**Modulo/Min/Max**:
- `fmod(x: f64, y: f64) f64`
- `remainder(x: f64, y: f64) f64`
- `fmin(x: f64, y: f64) f64`
- `fmax(x: f64, y: f64) f64`
- `copysign(x: f64, y: f64) f64`

---

## Time

**File**: `stdlib/time.zen`

```zen
Duration: {
    seconds: i64,
    nanos: i64
}

Instant: {
    seconds: i64,
    nanos: i64
}
```

**Functions**:
- `duration_from_secs(secs: i64) Duration`
- `duration_from_millis(millis: i64) Duration`
- `duration_as_secs(d: Duration) i64`
- `duration_as_millis(d: Duration) i64`
- `duration_add(a: Duration, b: Duration) Duration`
- `now() Instant` - Get current time
- `elapsed(start: Instant) Duration` - Time since
- `sleep(d: Duration)` - Block thread
- `sleep_millis(millis: i64)`
- `sleep_secs(secs: i64)`

---

## Random

**File**: `stdlib/random.zen`

Linear Congruential Generator (LCG) based pseudo-random numbers.

```zen
Rng: {
    state: i64
}

rng_new(seed: i64) Rng
rng_next(rng: *Rng) i64
rng_next_i32(rng: *Rng) i32
rng_next_u32(rng: *Rng) u32
rng_next_f64(rng: *Rng) f64  // [0.0, 1.0)
rng_next_bounded(rng: *Rng, max: i64) i64  // [0, max)
random() i64  // Global convenience
random_bounded(max: i64) i64
```

---

## Error Handling

**File**: `stdlib/error.zen`

```zen
ErrorLevel:
    Warning,
    Error,
    Fatal

Error: {
    level: ErrorLevel,
    message: String,
    code: i32
}
```

**Functions**:
- `error_new(message: String, code: i32) Error`
- `warning_new(message: String, code: i32) Error`
- `fatal_new(message: String, code: i32) Error`
- `is_warning(e: Error) bool`
- `is_error(e: Error) bool`
- `is_fatal(e: Error) bool`

---

## Testing

**File**: `stdlib/testing/runner.zen`

```zen
assert(condition: bool, message: String) void
assert_eq<T>(a: T, b: T, message: String) void
assert_ne<T>(a: T, b: T, message: String) void
assert_not(condition: bool, message: String) void
run_test(name: String, test_fn: () void) TestResult
```

---

## File System (Stubs)

**File**: `stdlib/fs/fs.zen`

All functions return `Result<T, String>` with "Not implemented" errors until FFI is complete.

```zen
open(path: String, mode: String) Result<File, String>
close(f: File) Result<void, String>
read(f: File, buffer: *u8, size: usize) Result<usize, String>
write(f: File, buffer: *u8, size: usize) Result<usize, String>
read_all(path: String) Result<String, String>
write_all(path: String, contents: String) Result<void, String>
exists(path: String) bool
file_size(path: String) Result<usize, String>
delete(path: String) Result<void, String>
mkdir(path: String) Result<void, String>
read_dir(path: String) Result<Vec<String>, String>
```

---

## Networking (Stubs)

**File**: `stdlib/net/net.zen`

All functions return `Result<T, String>` with "Not implemented" errors until FFI is complete.

```zen
tcp_socket(addr: SocketAddr) Result<TcpSocket, String>
tcp_listen(addr: SocketAddr) Result<TcpListener, String>
tcp_accept(listener: TcpListener) Result<TcpSocket, String>
tcp_send(socket: TcpSocket, buffer: *u8, size: usize) Result<usize, String>
tcp_recv(socket: TcpSocket, buffer: *u8, size: usize) Result<usize, String>
tcp_close(socket: TcpSocket) Result<void, String>

udp_socket(addr: SocketAddr) Result<UdpSocket, String>
udp_send(socket: UdpSocket, buffer: *u8, size: usize) Result<usize, String>
udp_recv(socket: UdpSocket, buffer: *u8, size: usize) Result<usize, String>
udp_close(socket: UdpSocket) Result<void, String>
```

---

## FFI (Foreign Function Interface)

**File**: `stdlib/ffi/ffi.zen`

C interoperability and dynamic loading.

```zen
CFuncPtr: {
    ptr: *u8
}

CLibrary: {
    handle: *u8,
    path: String
}

load_library(path: String) Result<CLibrary, String>
get_function(lib: CLibrary, symbol: String) Result<CFuncPtr, String>
call_cfunc(func: CFuncPtr, args: *u8) *u8
unload_library(lib: CLibrary) void
inline_c(code: String) void
```

---

## Implementation Notes

### Design Principles

1. **No Null Pointers** - Use `Option<T>` instead
2. **Type Safety** - Generic types with bounds checking
3. **Explicit Ownership** - Use `Ptr<T>` for owned heap allocations
4. **Error Handling** - Use `Result<T, E>` instead of exceptions
5. **Allocator Aware** - All allocations go through `Allocator` trait

### Known Limitations

1. **Load/Store Operations** - Zen needs `load(addr: *T) T` and `store(addr: *T, val: T)` primitives
   - Currently manually copying bytes with loops
   - This is inefficient and should be added to compiler intrinsics

2. **Type Casting** - Limited casting syntax
   - Cannot easily cast between generic types
   - Workaround: Use raw pointers and addresses

3. **Memory Copy** - No builtin memcpy equivalent in Zen
   - Doing byte-by-byte copies in loops
   - Should add `compiler.memcpy()` intrinsic

4. **Collections** - Simplified implementations
   - HashMap uses linear probing (not proper hash table)
   - Set and Queue do naive O(n) operations
   - These should be optimized with hash functions

### TODO - Missing Primitives

Priority order for next phase:

1. **Load/Store** - Critical for all data structure operations
   ```zen
   load<T>(addr: *T) T
   store<T>(addr: *T, value: T) void
   ```

2. **Memcpy** - For efficient bulk copying
   ```zen
   memcpy(dst: *u8, src: *u8, size: usize) void
   ```

3. **Hash Function** - For proper HashMap
   ```zen
   hash(data: *u8, size: usize) u64
   ```

4. **Type Equality** - For comparing generic types
   ```zen
   eq<T>(a: T, b: T) bool
   ```

---

## Usage Examples

### Working with Option
```zen
maybe_value: Option<i32> = Option.Some(42)

value = maybe_value ?
    | Some(v) { v }
    | None { 0 }
```

### Working with Result
```zen
read_number = (s: String) Result<i32, String> {
    // Try to parse
    return Result.Ok(42)
}

result = read_number("123")
result ?
    | Ok(n) { io.println("Parsed: ${n}") }
    | Err(e) { io.eprintln("Error: ${e}") }
```

### Allocating Memory
```zen
alloc = gpa_new()
ptr = gpa_allocate(alloc, 1024)
if ptr != (0 as *u8) {
    // Use memory
    gpa_deallocate(alloc, ptr, 1024)
}
```

### Growing Vector
```zen
v = vec_new()
elem: i32 = 42
elem_size = compiler.sizeof(i32)
elem_addr = &elem as *u8

vec_push(&v, elem_size, elem_addr)
vec_push(&v, elem_size, elem_addr)
```

---

## Next Steps

1. Implement load/store primitives in compiler
2. Add proper memcpy intrinsic
3. Optimize HashMap with chaining or better probing
4. Add String interpolation support
5. Implement proper Pattern matching for Result/Option
6. Complete FFI bindings for fs, net modules
7. Add concurrent primitives (Mutex, Channel, Actor)
