; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [17 x i8] c"Hello from Zen!\0A\00", align 1
@str.1 = private unnamed_addr constant [40 x i8] c"This demo shows correct import syntax.\0A\00", align 1
@str.2 = private unnamed_addr constant [8 x i8] c"Value: \00", align 1
@int_format = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.3 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.4 = private unnamed_addr constant [54 x i8] c"Imports are at module level, not in comptime blocks.\0A\00", align 1
@str.5 = private unnamed_addr constant [40 x i8] c"Comptime is only for meta-programming.\0A\00", align 1
@str.6 = private unnamed_addr constant [24 x i8] c"Hello from a function!\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %value = alloca i32, align 4
  store i32 42, ptr %value, align 4
  %printf_call2 = call i32 (ptr, ...) @printf(ptr @str.2)
  %value3 = load i32, ptr %value, align 4
  %printf_int_call = call i32 (ptr, ...) @printf(ptr @int_format, i32 %value3)
  %printf_call4 = call i32 (ptr, ...) @printf(ptr @str.3)
  %printf_call5 = call i32 (ptr, ...) @printf(ptr @str.4)
  %printf_call6 = call i32 (ptr, ...) @printf(ptr @str.5)
  ret i32 0
}

define void @greet() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str.6)
  ret void
}

declare i32 @printf(ptr, ...)
