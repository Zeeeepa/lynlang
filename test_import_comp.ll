; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [33 x i8] c"=== Zen Import System Test ===\0A\0A\00", align 1
@str.1 = private unnamed_addr constant [24 x i8] c"Testing math.abs(-42): \00", align 1
@int_format = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.2 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.3 = private unnamed_addr constant [23 x i8] c"Testing IO functions: \00", align 1
@str.4 = private unnamed_addr constant [7 x i8] c"text, \00", align 1
@int_format.5 = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.6 = private unnamed_addr constant [8 x i8] c", done\0A\00", align 1
@str.7 = private unnamed_addr constant [36 x i8] c"Core module: imported successfully\0A\00", align 1
@str.8 = private unnamed_addr constant [20 x i8] c"\0AAll tests passed!\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %result = alloca i32, align 4
  store i32 42, ptr %result, align 4
  %result2 = load i32, ptr %result, align 4
  %printf_int_call = call i32 (ptr, ...) @printf(ptr @int_format, i32 %result2)
  %printf_call3 = call i32 (ptr, ...) @printf(ptr @str.2)
  %printf_call4 = call i32 (ptr, ...) @printf(ptr @str.3)
  %printf_call5 = call i32 (ptr, ...) @printf(ptr @str.4)
  %printf_int_call6 = call i32 (ptr, ...) @printf(ptr @int_format.5, i32 123)
  %printf_call7 = call i32 (ptr, ...) @printf(ptr @str.6)
  %printf_call8 = call i32 (ptr, ...) @printf(ptr @str.7)
  %printf_call9 = call i32 (ptr, ...) @printf(ptr @str.8)
  ret i32 0
}

declare i32 @printf(ptr, ...)
