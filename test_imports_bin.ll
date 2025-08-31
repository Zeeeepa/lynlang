; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [17 x i8] c"Hello from Zen!\0A\00", align 1
@str.1 = private unnamed_addr constant [36 x i8] c"\E2\9C\85 Imports work without comptime!\0A\00", align 1
@str.2 = private unnamed_addr constant [46 x i8] c"\E2\9C\85 Clean syntax: identifier := @module.path\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %printf_call2 = call i32 (ptr, ...) @printf(ptr @str.2)
  ret i32 0
}

declare i32 @printf(ptr, ...)
