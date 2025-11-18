//! Enum variant compilation with heap allocation for nested generics
//!
//! This module handles compilation of enum variants (e.g., `Result.Ok(value)`, `Option.Some(x)`).
//! 
//! ## Heap Allocation Strategy for Nested Enums
//!
//! **Why heap allocation?** Nested enum types like `Result<Option<i32>, E>` or `Result<Result<T, E1>, E2>`
//! require careful memory management to avoid stack overflow and preserve nested payloads correctly.
//!
//! **The Problem:**
//! - Stack-allocated nested enum structs can cause stack overflow with deep nesting
//! - Nested payloads need to persist beyond the current stack frame
//! - Pattern matching on nested enums requires stable memory addresses
//!
//! **The Solution:**
//! - Detect nested enum variants (enum containing another enum as payload)
//! - Use `malloc()` to heap-allocate nested enum structs
//! - Store pointers to heap memory in the outer enum's payload field
//! - This ensures nested payloads are preserved and accessible during pattern matching
//!
//! **Example:** `Result.Ok(Option.Some(42))` creates:
//!   1. Heap-allocate `Option.Some(42)` struct → returns pointer
//!   2. Heap-allocate `Result.Ok(pointer)` struct → stores pointer to Option struct
//!   3. Both structs persist on the heap, enabling recursive pattern matching
//!
//! This complexity is necessary to support Zen's generic type system with nested enums.

use super::super::LLVMCompiler;
use crate::ast::{AstType, Expression};
use crate::error::CompileError;
use inkwell::values::BasicValueEnum;
use super::super::symbols;

pub fn compile_enum_variant<'ctx>(
        compiler: &mut LLVMCompiler<'ctx>,
        enum_name: &str,
        variant: &str,
        payload: &Option<Box<Expression>>,
    ) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // eprintln!("[DEBUG] compile_enum_variant: {}.{}, has_payload: {}", enum_name, variant, payload.is_some());
        // Save the current generic context before potentially overwriting it with nested compilation
        let saved_ok_type = compiler.generic_type_context.get("Result_Ok_Type").cloned();
        let saved_err_type = compiler.generic_type_context.get("Result_Err_Type").cloned();
        
        // Track Result<T, E> type information when compiling Result variants
        // This happens BEFORE we compile the payload, so we know what type this Result should be
        if enum_name == "Result" && payload.is_some() {
            if let Some(ref payload_expr) = payload {
                let payload_type = super::inference::infer_expression_type(compiler, payload_expr);
                if let Ok(t) = payload_type {
                    let key = if variant == "Ok" {
                        "Result_Ok_Type".to_string()
                    } else {
                        "Result_Err_Type".to_string()
                    };
        // eprintln!("[DEBUG TRACK] Setting {} = {:?}", key, t);
                    compiler.track_generic_type(key.clone(), t.clone());
                    // Also track nested generics recursively
                    if matches!(t, AstType::Generic { .. }) {
                        let prefix = if variant == "Ok" { "Result_Ok" } else { "Result_Err" };
                        compiler.track_complex_generic(&t, prefix);
                        // Use the generic tracker for better nested type handling
                        compiler.generic_tracker.track_generic_type(&t, prefix);
                    }
                    // Track custom enum types too
                    if matches!(t, AstType::EnumType { .. }) {
                        let prefix = if variant == "Ok" { "Result_Ok" } else { "Result_Err" };
                        compiler.generic_tracker.track_generic_type(&t, prefix);
                    }
                }
            }
        }

        // Track Option<T> type information when compiling Option variants
        if enum_name == "Option" && payload.is_some() {
            if let Some(ref payload_expr) = payload {
                let payload_type = super::inference::infer_expression_type(compiler, payload_expr);
                if let Ok(t) = payload_type {
                    compiler.track_generic_type("Option_Some_Type".to_string(), t.clone());
                    // Also track nested generics recursively
                    if matches!(t, AstType::Generic { .. }) {
                        compiler.track_complex_generic(&t, "Option_Some");
                    }
                    // Track DynVec types
                    if matches!(t, AstType::DynVec { .. }) {
                        compiler.generic_tracker.track_generic_type(&t, "Option_Some");
                    }
                    // Track custom enum types too
                    if matches!(t, AstType::EnumType { .. }) {
                        compiler.generic_tracker.track_generic_type(&t, "Option_Some");
                    }
                }
            }
        }

        // Helper: Find enum info and variant tag from symbol table
        let find_enum_info = |compiler: &LLVMCompiler<'ctx>, enum_name: &str, variant: &str| -> Option<(symbols::EnumInfo, u64)> {
            if !enum_name.is_empty() {
                // Try direct lookup by enum name
                if let Some(symbols::Symbol::EnumType(info)) = compiler.symbols.lookup(enum_name) {
                    if let Some(&tag) = info.variant_indices.get(variant) {
                        return Some((info.clone(), tag));
                    }
                }
            } else {
                // Search all enums for one containing this variant
                for symbol in compiler.symbols.all_symbols() {
                    if let symbols::Symbol::EnumType(info) = symbol {
                        if let Some(&tag) = info.variant_indices.get(variant) {
                            return Some((info.clone(), tag));
                        }
                    }
                }
            }
            None
        };

        // Look up the enum info from the symbol table
        let (enum_info, tag) = if let Some((info, tag_val)) = find_enum_info(compiler, enum_name, variant) {
            (Some(info), tag_val)
        } else {
            // If enum not found, infer enum name from variant for common types
            let inferred_enum = match variant {
                "Ok" | "Err" => Some("Result"),
                "Some" | "None" => Some("Option"),
                _ => None,
            };

            if let Some(inferred_name) = inferred_enum {
                // Try lookup with inferred name
                if let Some((info, tag_val)) = find_enum_info(compiler, inferred_name, variant) {
                    (Some(info), tag_val)
                } else {
                    return Err(CompileError::UndeclaredVariable(
                        format!("Enum variant '{}' not found. Expected enum '{}' to be registered.", variant, inferred_name),
                        None,
                    ));
                }
            } else {
                return Err(CompileError::UndeclaredVariable(
                    format!("Enum variant '{}' not found in symbol table. Cannot infer enum type.", variant),
                    None,
                ));
            }
        };

        // Use the enum info from symbol table - we should always have it at this point
        let enum_info = enum_info.expect("Enum info should be found by this point");
        let tag_val = compiler.context.i64_type().const_int(tag, false);
        
        // Use the enum's LLVM type from the symbol table
        let enum_struct_type = enum_info.llvm_type;

        let alloca = compiler.builder.build_alloca(
            enum_struct_type,
            &format!("{}_{}_enum_tmp", enum_name, variant),
        )?;
        let tag_ptr =
            compiler.builder
                .build_struct_gep(enum_struct_type, alloca, 0, "tag_ptr")?;
        compiler.builder.build_store(tag_ptr, tag_val)?;

        // Handle payload if present
        if let Some(expr) = payload {
            // eprintln!("[DEBUG] Compiling payload expression for {}.{}", enum_name, variant);
            
            // Check if the payload is itself an enum variant (nested case)
            let is_nested_enum = match expr.as_ref() {
                Expression::EnumVariant { .. } => true,
                Expression::MemberAccess { member, .. } => {
                    member == "Ok" || member == "Err" || member == "Some" || member == "None"
                }
                _ => false,
            };
            
            if is_nested_enum {
                // eprintln!("[DEBUG] *** NESTED ENUM DETECTED *** Payload is a nested enum variant!");
            }
            
            let compiled = compiler.compile_expression(expr)?;
            
            // Check if enum struct has payload field (some enums might not have payloads)
            if enum_struct_type.count_fields() > 1 {
                let payload_ptr = compiler.builder.build_struct_gep(
                    enum_struct_type,
                    alloca,
                    1,
                    "payload_ptr",
                )?;

                // Store payload as pointer
                // Check if the payload is an enum struct itself (for nested generics)
                let payload_value = if compiled.is_struct_value() {
                            // For enum structs (like nested Result/Option), we need special handling
                            let struct_val = compiled.into_struct_value();
                            let struct_type = struct_val.get_type();
                            
                            // Check if this is a Result/Option enum struct (has 2 fields: discriminant + payload ptr)
                            // These need heap allocation to preserve nested payloads
                            if struct_type.count_fields() == 2 {
        // eprintln!("[DEBUG] Heap allocating nested enum struct as payload for {}.{}", enum_name, variant);
                                // This looks like an enum struct - heap allocate it
                                let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
                                    let i64_type = compiler.context.i64_type();
                                    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
                                    let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                    compiler.module.add_function("malloc", malloc_type, None)
                                });
                                
                                // Allocate 16 bytes for the enum struct (8 for discriminant + 8 for pointer)
                                let struct_size = compiler.context.i64_type().const_int(16, false);
                                let malloc_call = compiler.builder.build_call(malloc_fn, &[struct_size.into()], "heap_enum")?;
                                let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                                
                                // Store the entire struct - this preserves the discriminant and payload pointer
                                compiler.builder.build_store(heap_ptr, struct_val)?;
                                
                                // CRITICAL FIX: We need to ensure the nested payload's payload is also heap-allocated!
                                // Extract the discriminant and payload pointer from the struct
                                let discriminant = compiler.builder.build_extract_value(struct_val, 0, "nested_disc")?;
                                let payload_ptr = compiler.builder.build_extract_value(struct_val, 1, "nested_payload_ptr")?;
                                
                                
                                // CRITICAL: For nested enums, we need to ensure the inner payload is persistent
                                // The inner Result.Ok(42) has its payload (42) on the heap, but we need to verify
                                if payload_ptr.is_pointer_value() && discriminant.is_int_value() {
                                    let disc_val = discriminant.into_int_value();
                                    // Check if this is an Ok/Some variant (discriminant == 0)
                                    let is_ok_or_some = disc_val.is_const() && disc_val.get_zero_extended_constant() == Some(0);
                                    
                                    if is_ok_or_some {
                                        // The payload pointer should point to heap memory
                                        // No additional work needed - the nested compile_enum_variant
                                        // should have already heap-allocated the inner payload
                                    }
                                }
                                
                                heap_ptr.into()
                            } else {
                                // Not an enum struct, just heap-allocate and copy
                                let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
                                    let i64_type = compiler.context.i64_type();
                                    let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
                                    let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                    compiler.module.add_function("malloc", malloc_type, None)
                                });
                                
                                let size = compiler.context.i64_type().const_int(16, false);
                                let malloc_call = compiler.builder.build_call(malloc_fn, &[size.into()], "heap_struct")?;
                                let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                                compiler.builder.build_store(heap_ptr, struct_val)?;
                                heap_ptr.into()
                            }
                        } else if compiled.is_pointer_value() {
                            compiled
                        } else {
                            // For simple values (i32, i64, etc), heap-allocate to ensure they persist
                            let malloc_fn = compiler.module.get_function("malloc").unwrap_or_else(|| {
                                let i64_type = compiler.context.i64_type();
                                let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
                                let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
                                compiler.module.add_function("malloc", malloc_type, None)
                            });
                            
                            // Allocate enough space for the value (8 bytes should cover most primitives)
                            let size = compiler.context.i64_type().const_int(8, false);
                            let malloc_call = compiler.builder.build_call(malloc_fn, &[size.into()], "heap_payload")?;
                            let heap_ptr = malloc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                            
                            compiler.builder.build_store(heap_ptr, compiled)?;
                            heap_ptr.into()
                        };
                compiler.builder.build_store(payload_ptr, payload_value)?;
            }
        } else {
            // For variants without payload (like None), store null pointer if struct has payload field
            if enum_struct_type.count_fields() > 1 {
                let ptr_type = compiler.context.ptr_type(inkwell::AddressSpace::default());
                let payload_ptr = compiler.builder.build_struct_gep(
                    enum_struct_type,
                    alloca,
                    1,
                    "payload_ptr",
                )?;
                let null_ptr = ptr_type.const_null();
                compiler.builder.build_store(payload_ptr, null_ptr)?;
            }
        }

        let loaded = compiler.builder.build_load(
            enum_struct_type,
            alloca,
            &format!("{}_{}_enum_val", enum_name, variant),
        )?;
        Ok(loaded)
    }
