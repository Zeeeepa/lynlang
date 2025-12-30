// Result and Option type metadata
//
// Note: Result<T, E> and Option<T> are now defined in stdlib/result.zen and stdlib/option.zen
// using compiler intrinsics. The AST construction helpers that were here have been removed
// as they were unused - the actual Result/Option handling is done via the well_known module.
//
// See:
// - src/well_known.rs for Result/Option type detection
// - stdlib/result.zen for the Zen implementation
// - stdlib/option.zen for the Zen implementation
