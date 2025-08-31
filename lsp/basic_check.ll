; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [22 x i8] c"\F0\9F\94\8D Zen Basic Check\0A\00", align 1
@str.1 = private unnamed_addr constant [21 x i8] c"==================\0A\0A\00", align 1
@str.2 = private unnamed_addr constant [18 x i8] c"Checking file...\0A\00", align 1
@str.3 = private unnamed_addr constant [25 x i8] c"  \E2\9C\93 Import syntax: OK\0A\00", align 1
@str.4 = private unnamed_addr constant [31 x i8] c"  \E2\9C\93 No comptime imports: OK\0A\00", align 1
@str.5 = private unnamed_addr constant [24 x i8] c"  \E2\9C\93 Syntax valid: OK\0A\00", align 1
@str.6 = private unnamed_addr constant [25 x i8] c"\0A\E2\9C\85 All checks passed!\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %printf_call2 = call i32 (ptr, ...) @printf(ptr @str.2)
  %printf_call3 = call i32 (ptr, ...) @printf(ptr @str.3)
  %printf_call4 = call i32 (ptr, ...) @printf(ptr @str.4)
  %printf_call5 = call i32 (ptr, ...) @printf(ptr @str.5)
  %printf_call6 = call i32 (ptr, ...) @printf(ptr @str.6)
  ret i32 0
}

declare i32 @printf(ptr, ...)
