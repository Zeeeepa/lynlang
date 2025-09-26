Check the compilation output more thoroughly - the issue is that the typechecker is seeing Result<I32, _> when it should see Result<Result<I32, String>, String>.

The LLVM compiler seems to correctly track it (we saw "[DEBUG VAR] Variable outer2 inferred as Result<Generic { name: \"Result\", type_args: [I32, String] }, String>"), but the type checker has a simpler type tracking mechanism.

The fundamental issue is that the type checker and the LLVM compiler have separate type tracking systems, and the type checker is not as sophisticated in tracking nested generics.
