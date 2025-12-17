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

        _ => "Zen language element",
    }
}
