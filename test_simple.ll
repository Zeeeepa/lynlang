; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [17 x i8] c"Hello from Zen!\0A\00", align 1
@int_format = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.1 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %result = alloca i32, align 4
  store i32 42, ptr %result, align 4
  %result1 = load i32, ptr %result, align 4
  %printf_int_call = call i32 (ptr, ...) @printf(ptr @int_format, i32 %result1)
  %printf_call2 = call i32 (ptr, ...) @printf(ptr @str.1)
  ret i32 0
}

declare i32 @printf(ptr, ...)
