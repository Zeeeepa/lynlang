// Built-in types hover information

/// Get hover text for built-in types and keywords
pub fn get_builtin_hover_text(symbol_name: &str) -> &'static str {
    match symbol_name {
        // Primitive integer types
        "i8" => "```zen\ni8\n```\n\n**Signed 8-bit integer**\n- Range: -128 to 127\n- Size: 1 byte",
        "i16" => "```zen\ni16\n```\n\n**Signed 16-bit integer**\n- Range: -32,768 to 32,767\n- Size: 2 bytes",
        "i32" => "```zen\ni32\n```\n\n**Signed 32-bit integer**\n- Range: -2,147,483,648 to 2,147,483,647\n- Size: 4 bytes",
        "i64" => "```zen\ni64\n```\n\n**Signed 64-bit integer**\n- Range: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807\n- Size: 8 bytes",
        "u8" => "```zen\nu8\n```\n\n**Unsigned 8-bit integer**\n- Range: 0 to 255\n- Size: 1 byte",
        "u16" => "```zen\nu16\n```\n\n**Unsigned 16-bit integer**\n- Range: 0 to 65,535\n- Size: 2 bytes",
        "u32" => "```zen\nu32\n```\n\n**Unsigned 32-bit integer**\n- Range: 0 to 4,294,967,295\n- Size: 4 bytes",
        "u64" => "```zen\nu64\n```\n\n**Unsigned 64-bit integer**\n- Range: 0 to 18,446,744,073,709,551,615\n- Size: 8 bytes",
        "usize" => "```zen\nusize\n```\n\n**Pointer-sized unsigned integer**\n- Size: Platform dependent (4 or 8 bytes)\n- Used for array indexing and memory offsets",

        // Floating point types
        "f32" => "```zen\nf32\n```\n\n**32-bit floating point**\n- Precision: ~7 decimal digits\n- Size: 4 bytes\n- IEEE 754 single precision",
        "f64" => "```zen\nf64\n```\n\n**64-bit floating point**\n- Precision: ~15 decimal digits\n- Size: 8 bytes\n- IEEE 754 double precision",

        // Boolean
        "bool" => "```zen\nbool\n```\n\n**Boolean type**\n- Values: `true` or `false`\n- Size: 1 byte",

        // Void
        "void" => "```zen\nvoid\n```\n\n**Void type**\n- Represents the absence of a value\n- Used as return type for functions with no return",

        // Option and Result
        "Option" => "```zen\nOption<T>:\n    Some: T,\n    None\n```\n\n**Optional value type**\n- Represents a value that may or may not exist\n- No null/nil in Zen!",
        "Result" => "```zen\nResult<T, E>:\n    Ok: T,\n    Err: E\n```\n\n**Result type for error handling**\n- Represents success (Ok) or failure (Err)\n- Use `.raise()` for error propagation",

        // Collections
        "HashMap" => "```zen\nHashMap<K, V>\n```\n\n**Hash map collection**\n- Key-value storage with O(1) average lookup\n- Requires allocator",
        "DynVec" => "```zen\nDynVec<T>\n```\n\n**Dynamic vector**\n- Growable array\n- Requires allocator",
        "Vec" => "```zen\nVec<T, size>\n```\n\n**Fixed-size vector**\n- Stack-allocated\n- Compile-time size\n- No allocator needed",
        "Array" => "```zen\nArray<T>\n```\n\n**Dynamic array**\n- Requires allocator",

        // String types
        "String" => "```zen\nString\n```\n\n**Dynamic string type**\n- Mutable, heap-allocated\n- Requires allocator",
        "StaticString" => "```zen\nStaticString\n```\n\n**Static string type**\n- Immutable, compile-time\n- No allocator needed",

        // Keywords
        "loop" => "```zen\nloop() { ... }\nloop((handle) { ... })\n(range).loop((i) { ... })\n```\n\n**Loop construct**\n- Internal state management\n- Can provide control handle or iteration values",
        "return" => "```zen\nreturn expr\n```\n\n**Return statement**\n- Returns a value from a function",
        "break" => "```zen\nbreak\n```\n\n**Break statement**\n- Exits the current loop",
        "continue" => "```zen\ncontinue\n```\n\n**Continue statement**\n- Skips to the next loop iteration",

        // Error handling
        "raise" => "```zen\nexpr.raise()\n```\n\n**Error propagation**\n- Unwraps Result<T, E> or returns Err early\n- Equivalent to Rust's `?` operator",

        "raw_allocate" => "```zen\n@std.compiler.raw_allocate(size: usize) -> *u8\n```\n\n**Memory allocation**\n- Allocates raw memory using malloc\n- Returns null if allocation fails",
        "raw_deallocate" => "```zen\n@std.compiler.raw_deallocate(ptr: *u8, size: usize) -> void\n```\n\n**Memory deallocation**\n- Deallocates memory previously allocated with raw_allocate",
        "raw_reallocate" => "```zen\n@std.compiler.raw_reallocate(ptr: *u8, old_size: usize, new_size: usize) -> *u8\n```\n\n**Memory reallocation**\n- Reallocates memory to new size, preserving data",
        "gep" => "```zen\n@std.compiler.gep(base_ptr: *u8, offset: i64) -> *u8\n```\n\n**GetElementPointer**\n- Byte-level pointer arithmetic\n- Offset can be negative",
        "gep_struct" => "```zen\n@std.compiler.gep_struct(struct_ptr: *u8, field_index: i32) -> *u8\n```\n\n**Struct field access**\n- Returns pointer to field at given index",
        "null_ptr" | "nullptr" => "```zen\n@std.compiler.null_ptr() -> *u8\n```\n\n**Null pointer constant**\n- Returns a null pointer (address 0)",
        "sizeof" => "```zen\n@std.compiler.sizeof<T>() -> usize\n```\n\n**Type size**\n- Returns the size of a type in bytes",
        "alignof" => "```zen\n@std.compiler.alignof<T>() -> usize\n```\n\n**Type alignment**\n- Returns the alignment of a type in bytes",
        "load" => "```zen\n@std.compiler.load(ptr: *u8) -> T\n```\n\n**Memory load**\n- Load a value from a pointer",
        "store" => "```zen\n@std.compiler.store(ptr: *u8, value: T) -> void\n```\n\n**Memory store**\n- Store a value to a pointer",
        "memcpy" => "```zen\n@std.compiler.memcpy(dest: *u8, src: *u8, size: usize) -> void\n```\n\n**Memory copy**\n- Copy bytes (regions must not overlap)",
        "memmove" => "```zen\n@std.compiler.memmove(dest: *u8, src: *u8, size: usize) -> void\n```\n\n**Memory move**\n- Copy bytes (safe for overlapping regions)",
        "memset" => "```zen\n@std.compiler.memset(dest: *u8, value: u8, size: usize) -> void\n```\n\n**Memory set**\n- Set all bytes in memory to a value",
        "memcmp" => "```zen\n@std.compiler.memcmp(ptr1: *u8, ptr2: *u8, size: usize) -> i32\n```\n\n**Memory compare**\n- Compare bytes in memory",
        "discriminant" => "```zen\n@std.compiler.discriminant(enum_ptr: *u8) -> i32\n```\n\n**Enum discriminant**\n- Reads the variant tag from an enum",
        "set_discriminant" => "```zen\n@std.compiler.set_discriminant(enum_ptr: *u8, discriminant: i32) -> void\n```\n\n**Set enum discriminant**\n- Sets the variant tag of an enum",
        "get_payload" => "```zen\n@std.compiler.get_payload(enum_ptr: *u8) -> *u8\n```\n\n**Get enum payload**\n- Returns pointer to payload data",
        "atomic_load" => "```zen\n@std.compiler.atomic_load(ptr: *u64) -> u64\n```\n\n**Atomic load**\n- Atomically load a value",
        "atomic_store" => "```zen\n@std.compiler.atomic_store(ptr: *u64, value: u64) -> void\n```\n\n**Atomic store**\n- Atomically store a value",
        "atomic_add" => "```zen\n@std.compiler.atomic_add(ptr: *u64, value: u64) -> u64\n```\n\n**Atomic add**\n- Atomically add and return old value",
        "atomic_cas" => "```zen\n@std.compiler.atomic_cas(ptr: *u64, expected: u64, new: u64) -> bool\n```\n\n**Compare-and-swap**\n- Returns true if swap succeeded",
        "bswap16" | "bswap32" | "bswap64" => "```zen\n@std.compiler.bswap*(value) -> same_type\n```\n\n**Byte swap**\n- Swap bytes for endian conversion",
        "ctlz" => "```zen\n@std.compiler.ctlz(value: u64) -> u64\n```\n\n**Count leading zeros**\n- Returns number of leading zero bits",
        "cttz" => "```zen\n@std.compiler.cttz(value: u64) -> u64\n```\n\n**Count trailing zeros**\n- Returns number of trailing zero bits",
        "ctpop" => "```zen\n@std.compiler.ctpop(value: u64) -> u64\n```\n\n**Population count**\n- Returns number of bits set to 1",
        "trap" => "```zen\n@std.compiler.trap() -> void\n```\n\n**Trap**\n- Trigger a trap/abort",
        "unreachable" => "```zen\n@std.compiler.unreachable() -> void\n```\n\n**Unreachable**\n- Mark code as unreachable (UB if reached)",

        _ => "Zen language element",
    }
}
