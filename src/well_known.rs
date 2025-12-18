//! Registry for well-known types that have special compiler semantics.
//!
//! This module centralizes the recognition of types like Option, Result, Ptr, etc.
//! that require special handling in the typechecker and codegen. By using this
//! registry instead of hardcoded string comparisons, we:
//!
//! 1. Enable future self-hosting (a Zen compiler written in Zen can use the same pattern)
//! 2. Make the codebase more maintainable (one source of truth)
//! 3. Allow LSP "Go to definition" to work on stdlib types
//! 4. Allow adding new well-known types without changing compiler code

use std::collections::HashMap;

/// Well-known types that have special compiler semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WellKnownType {
    /// Option<T> - nullable type
    Option,
    /// Result<T, E> - error handling type
    Result,
    /// Ptr<T> - immutable pointer
    Ptr,
    /// MutPtr<T> - mutable pointer
    MutPtr,
    /// RawPtr<T> - raw/unsafe pointer
    RawPtr,
    /// String - dynamic string type
    String,
    /// Vec<T> - dynamic array (stack-allocated metadata)
    Vec,
    /// DynVec<T...> - dynamic vector with allocator
    DynVec,
    /// HashMap<K, V> - hash map collection
    HashMap,
    /// HashSet<T> - hash set collection
    HashSet,
}

/// Well-known enum variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WellKnownVariant {
    /// Option::Some(T)
    Some,
    /// Option::None
    None,
    /// Result::Ok(T)
    Ok,
    /// Result::Err(E)
    Err,
}

/// Registry of well-known types and their variants
#[derive(Debug, Clone)]
pub struct WellKnownTypes {
    /// Map from type name to well-known type
    types: HashMap<String, WellKnownType>,
    /// Map from variant name to (parent type, variant)
    variants: HashMap<String, (WellKnownType, WellKnownVariant)>,
}

impl WellKnownTypes {
    /// Create a new registry with all well-known types registered
    pub fn new() -> Self {
        let mut wkt = Self {
            types: HashMap::with_capacity(10),
            variants: HashMap::with_capacity(4),
        };

        // Register well-known types
        wkt.types.insert("Option".into(), WellKnownType::Option);
        wkt.types.insert("Result".into(), WellKnownType::Result);
        wkt.types.insert("Ptr".into(), WellKnownType::Ptr);
        wkt.types.insert("MutPtr".into(), WellKnownType::MutPtr);
        wkt.types.insert("RawPtr".into(), WellKnownType::RawPtr);
        wkt.types.insert("String".into(), WellKnownType::String);
        wkt.types.insert("Vec".into(), WellKnownType::Vec);
        wkt.types.insert("DynVec".into(), WellKnownType::DynVec);
        wkt.types.insert("HashMap".into(), WellKnownType::HashMap);
        wkt.types.insert("HashSet".into(), WellKnownType::HashSet);

        // Register well-known variants
        wkt.variants
            .insert("Some".into(), (WellKnownType::Option, WellKnownVariant::Some));
        wkt.variants
            .insert("None".into(), (WellKnownType::Option, WellKnownVariant::None));
        wkt.variants
            .insert("Ok".into(), (WellKnownType::Result, WellKnownVariant::Ok));
        wkt.variants
            .insert("Err".into(), (WellKnownType::Result, WellKnownVariant::Err));

        wkt
    }

    // ========================================================================
    // Type checks
    // ========================================================================

    /// Get the well-known type for a name, if any
    #[inline]
    pub fn get_type(&self, name: &str) -> Option<WellKnownType> {
        self.types.get(name).copied()
    }

    /// Check if a type name is Option
    #[inline]
    pub fn is_option(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::Option)
    }

    /// Check if a type name is Result
    #[inline]
    pub fn is_result(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::Result)
    }

    /// Check if a type name is any pointer type (Ptr, MutPtr, RawPtr)
    #[inline]
    pub fn is_ptr(&self, name: &str) -> bool {
        matches!(
            self.get_type(name),
            Some(WellKnownType::Ptr | WellKnownType::MutPtr | WellKnownType::RawPtr)
        )
    }

    /// Check if a type name is an immutable pointer (Ptr)
    #[inline]
    pub fn is_immutable_ptr(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::Ptr)
    }

    /// Check if a type name is a mutable pointer (MutPtr)
    #[inline]
    pub fn is_mutable_ptr(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::MutPtr)
    }

    /// Check if a type name is a raw pointer (RawPtr)
    #[inline]
    pub fn is_raw_ptr(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::RawPtr)
    }

    /// Get the pointer type kind if this is a pointer type
    #[inline]
    pub fn get_ptr_type(&self, name: &str) -> Option<WellKnownType> {
        match self.get_type(name) {
            Some(t @ (WellKnownType::Ptr | WellKnownType::MutPtr | WellKnownType::RawPtr)) => {
                Some(t)
            }
            _ => None,
        }
    }

    /// Check if a type name is Option or Result (types with success/failure variants)
    #[inline]
    pub fn is_option_or_result(&self, name: &str) -> bool {
        matches!(
            self.get_type(name),
            Some(WellKnownType::Option | WellKnownType::Result)
        )
    }

    /// Check if a type name is String
    #[inline]
    pub fn is_string(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::String)
    }

    /// Check if a type name is Vec
    #[inline]
    pub fn is_vec(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::Vec)
    }

    /// Check if a type name is DynVec
    #[inline]
    pub fn is_dyn_vec(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::DynVec)
    }

    /// Check if a type name is Vec or DynVec
    #[inline]
    pub fn is_vec_type(&self, name: &str) -> bool {
        matches!(
            self.get_type(name),
            Some(WellKnownType::Vec | WellKnownType::DynVec)
        )
    }

    /// Check if a type name is HashMap
    #[inline]
    pub fn is_hash_map(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::HashMap)
    }

    /// Check if a type name is HashSet
    #[inline]
    pub fn is_hash_set(&self, name: &str) -> bool {
        self.get_type(name) == Some(WellKnownType::HashSet)
    }

    /// Check if a type name is a collection type (Vec, DynVec, HashMap, HashSet)
    #[inline]
    pub fn is_collection(&self, name: &str) -> bool {
        matches!(
            self.get_type(name),
            Some(
                WellKnownType::Vec
                    | WellKnownType::DynVec
                    | WellKnownType::HashMap
                    | WellKnownType::HashSet
            )
        )
    }

    // ========================================================================
    // Variant checks
    // ========================================================================

    /// Get the well-known variant info for a name, if any
    #[inline]
    pub fn get_variant(&self, name: &str) -> Option<(WellKnownType, WellKnownVariant)> {
        self.variants.get(name).copied()
    }

    /// Check if a variant name belongs to Option (Some or None)
    #[inline]
    pub fn is_option_variant(&self, name: &str) -> bool {
        matches!(self.get_variant(name), Some((WellKnownType::Option, _)))
    }

    /// Check if a variant name belongs to Result (Ok or Err)
    #[inline]
    pub fn is_result_variant(&self, name: &str) -> bool {
        matches!(self.get_variant(name), Some((WellKnownType::Result, _)))
    }

    /// Check if a variant name is Some
    #[inline]
    pub fn is_some(&self, name: &str) -> bool {
        matches!(self.get_variant(name), Some((_, WellKnownVariant::Some)))
    }

    /// Check if a variant name is None
    #[inline]
    pub fn is_none(&self, name: &str) -> bool {
        matches!(self.get_variant(name), Some((_, WellKnownVariant::None)))
    }

    /// Check if a variant name is Ok
    #[inline]
    pub fn is_ok(&self, name: &str) -> bool {
        matches!(self.get_variant(name), Some((_, WellKnownVariant::Ok)))
    }

    /// Check if a variant name is Err
    #[inline]
    pub fn is_err(&self, name: &str) -> bool {
        matches!(self.get_variant(name), Some((_, WellKnownVariant::Err)))
    }

    /// Check if a variant is a "success" variant (Some or Ok)
    #[inline]
    pub fn is_success_variant(&self, name: &str) -> bool {
        matches!(
            self.get_variant(name),
            Some((_, WellKnownVariant::Some | WellKnownVariant::Ok))
        )
    }

    /// Check if a variant is a "failure" variant (None or Err)
    #[inline]
    pub fn is_failure_variant(&self, name: &str) -> bool {
        matches!(
            self.get_variant(name),
            Some((_, WellKnownVariant::None | WellKnownVariant::Err))
        )
    }

    /// Get the parent type for a variant
    #[inline]
    pub fn get_variant_parent(&self, variant_name: &str) -> Option<WellKnownType> {
        self.get_variant(variant_name).map(|(parent, _)| parent)
    }

    /// Get the canonical type name for a variant's parent
    #[inline]
    pub fn get_variant_parent_name(&self, variant_name: &str) -> Option<&'static str> {
        self.get_variant_parent(variant_name).map(|t| match t {
            WellKnownType::Option => "Option",
            WellKnownType::Result => "Result",
            WellKnownType::Ptr => "Ptr",
            WellKnownType::MutPtr => "MutPtr",
            WellKnownType::RawPtr => "RawPtr",
            WellKnownType::String => "String",
            WellKnownType::Vec => "Vec",
            WellKnownType::DynVec => "DynVec",
            WellKnownType::HashMap => "HashMap",
            WellKnownType::HashSet => "HashSet",
        })
    }

    // ========================================================================
    // Canonical name getters (for type construction)
    // ========================================================================

    /// Get the canonical name for Option type
    #[inline]
    pub fn option_name(&self) -> &'static str {
        "Option"
    }

    /// Get the canonical name for Result type
    #[inline]
    pub fn result_name(&self) -> &'static str {
        "Result"
    }

    /// Get the canonical name for Ptr type
    #[inline]
    pub fn ptr_name(&self) -> &'static str {
        "Ptr"
    }

    /// Get the canonical name for MutPtr type
    #[inline]
    pub fn mut_ptr_name(&self) -> &'static str {
        "MutPtr"
    }

    /// Get the canonical name for RawPtr type
    #[inline]
    pub fn raw_ptr_name(&self) -> &'static str {
        "RawPtr"
    }

    #[inline]
    pub fn string_name(&self) -> &'static str {
        "String"
    }

    #[inline]
    pub fn vec_name(&self) -> &'static str {
        "Vec"
    }

    #[inline]
    pub fn dyn_vec_name(&self) -> &'static str {
        "DynVec"
    }

    #[inline]
    pub fn hash_map_name(&self) -> &'static str {
        "HashMap"
    }

    #[inline]
    pub fn hash_set_name(&self) -> &'static str {
        "HashSet"
    }

    // ========================================================================
    // Variant name getters (for passing to get_variant_parent_name etc)
    // ========================================================================

    /// Get the canonical name for Some variant
    #[inline]
    pub fn some_name(&self) -> &'static str {
        "Some"
    }

    /// Get the canonical name for None variant
    #[inline]
    pub fn none_name(&self) -> &'static str {
        "None"
    }

    /// Get the canonical name for Ok variant
    #[inline]
    pub fn ok_name(&self) -> &'static str {
        "Ok"
    }

    /// Get the canonical name for Err variant
    #[inline]
    pub fn err_name(&self) -> &'static str {
        "Err"
    }

    // ========================================================================
    // Combined checks (useful for common patterns)
    // ========================================================================

    /// Check if name is a well-known type or variant
    #[inline]
    pub fn is_well_known(&self, name: &str) -> bool {
        self.types.contains_key(name) || self.variants.contains_key(name)
    }

    /// Get discriminant tag for a variant (for codegen)
    /// Returns 0 for success variants (Some, Ok), 1 for failure variants (None, Err)
    #[inline]
    pub fn get_variant_tag(&self, variant_name: &str) -> Option<u64> {
        match self.get_variant(variant_name) {
            Some((_, WellKnownVariant::Some)) => Some(0),
            Some((_, WellKnownVariant::Ok)) => Some(0),
            Some((_, WellKnownVariant::None)) => Some(1),
            Some((_, WellKnownVariant::Err)) => Some(1),
            None => None,
        }
    }
}

impl Default for WellKnownTypes {
    fn default() -> Self {
        Self::new()
    }
}

/// Global static instance for use in parser and other contexts where
/// threading through parameters is impractical
pub fn well_known() -> &'static WellKnownTypes {
    use std::sync::OnceLock;
    static INSTANCE: OnceLock<WellKnownTypes> = OnceLock::new();
    INSTANCE.get_or_init(WellKnownTypes::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_checks() {
        let wkt = WellKnownTypes::new();

        assert!(wkt.is_option("Option"));
        assert!(wkt.is_result("Result"));
        assert!(wkt.is_ptr("Ptr"));
        assert!(wkt.is_ptr("MutPtr"));
        assert!(wkt.is_ptr("RawPtr"));
        assert!(wkt.is_string("String"));
        assert!(wkt.is_vec("Vec"));
        assert!(wkt.is_dyn_vec("DynVec"));
        assert!(wkt.is_hash_map("HashMap"));
        assert!(wkt.is_hash_set("HashSet"));

        assert!(!wkt.is_option("Result"));
        assert!(!wkt.is_result("Option"));
        assert!(!wkt.is_ptr("Option"));
        assert!(!wkt.is_string("Vec"));
    }

    #[test]
    fn test_collection_checks() {
        let wkt = WellKnownTypes::new();

        assert!(wkt.is_collection("Vec"));
        assert!(wkt.is_collection("DynVec"));
        assert!(wkt.is_collection("HashMap"));
        assert!(wkt.is_collection("HashSet"));
        assert!(wkt.is_vec_type("Vec"));
        assert!(wkt.is_vec_type("DynVec"));

        assert!(!wkt.is_collection("Option"));
        assert!(!wkt.is_collection("String"));
        assert!(!wkt.is_vec_type("HashMap"));
    }

    #[test]
    fn test_variant_checks() {
        let wkt = WellKnownTypes::new();

        assert!(wkt.is_some("Some"));
        assert!(wkt.is_none("None"));
        assert!(wkt.is_ok("Ok"));
        assert!(wkt.is_err("Err"));

        assert!(wkt.is_option_variant("Some"));
        assert!(wkt.is_option_variant("None"));
        assert!(wkt.is_result_variant("Ok"));
        assert!(wkt.is_result_variant("Err"));

        assert!(!wkt.is_option_variant("Ok"));
        assert!(!wkt.is_result_variant("Some"));
    }

    #[test]
    fn test_variant_tags() {
        let wkt = WellKnownTypes::new();

        assert_eq!(wkt.get_variant_tag("Some"), Some(0));
        assert_eq!(wkt.get_variant_tag("Ok"), Some(0));
        assert_eq!(wkt.get_variant_tag("None"), Some(1));
        assert_eq!(wkt.get_variant_tag("Err"), Some(1));
        assert_eq!(wkt.get_variant_tag("Unknown"), None);
    }

    #[test]
    fn test_global_instance() {
        let wkt = well_known();
        assert!(wkt.is_option("Option"));
    }
}
