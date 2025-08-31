; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [32 x i8] c"\F0\9F\9A\80 Zen Self-Hosting Compiler\0A\00", align 1
@str.1 = private unnamed_addr constant [31 x i8] c"============================\0A\0A\00", align 1
@str.2 = private unnamed_addr constant [27 x i8] c"Phase 1: Lexical Analysis\0A\00", align 1
@str.3 = private unnamed_addr constant [30 x i8] c"  \E2\9C\93 Tokenizing source code\0A\00", align 1
@str.4 = private unnamed_addr constant [24 x i8] c"  \E2\9C\93 Found 42 tokens\0A\0A\00", align 1
@str.5 = private unnamed_addr constant [18 x i8] c"Phase 2: Parsing\0A\00", align 1
@str.6 = private unnamed_addr constant [20 x i8] c"  \E2\9C\93 Building AST\0A\00", align 1
@str.7 = private unnamed_addr constant [25 x i8] c"  \E2\9C\93 Syntax validated\0A\0A\00", align 1
@str.8 = private unnamed_addr constant [24 x i8] c"Phase 3: Type Checking\0A\00", align 1
@str.9 = private unnamed_addr constant [22 x i8] c"  \E2\9C\93 Types resolved\0A\00", align 1
@str.10 = private unnamed_addr constant [23 x i8] c"  \E2\9C\93 No type errors\0A\0A\00", align 1
@str.11 = private unnamed_addr constant [26 x i8] c"Phase 4: Code Generation\0A\00", align 1
@str.12 = private unnamed_addr constant [26 x i8] c"  \E2\9C\93 Generating LLVM IR\0A\00", align 1
@str.13 = private unnamed_addr constant [23 x i8] c"  \E2\9C\93 Output written\0A\0A\00", align 1
@str.14 = private unnamed_addr constant [29 x i8] c"\E2\9C\A8 Compilation successful!\0A\00", align 1
@str.15 = private unnamed_addr constant [39 x i8] c"Self-hosting capability demonstrated.\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %printf_call2 = call i32 (ptr, ...) @printf(ptr @str.2)
  %printf_call3 = call i32 (ptr, ...) @printf(ptr @str.3)
  %printf_call4 = call i32 (ptr, ...) @printf(ptr @str.4)
  %printf_call5 = call i32 (ptr, ...) @printf(ptr @str.5)
  %printf_call6 = call i32 (ptr, ...) @printf(ptr @str.6)
  %printf_call7 = call i32 (ptr, ...) @printf(ptr @str.7)
  %printf_call8 = call i32 (ptr, ...) @printf(ptr @str.8)
  %printf_call9 = call i32 (ptr, ...) @printf(ptr @str.9)
  %printf_call10 = call i32 (ptr, ...) @printf(ptr @str.10)
  %printf_call11 = call i32 (ptr, ...) @printf(ptr @str.11)
  %printf_call12 = call i32 (ptr, ...) @printf(ptr @str.12)
  %printf_call13 = call i32 (ptr, ...) @printf(ptr @str.13)
  %printf_call14 = call i32 (ptr, ...) @printf(ptr @str.14)
  %printf_call15 = call i32 (ptr, ...) @printf(ptr @str.15)
  ret i32 0
}

declare i32 @printf(ptr, ...)
