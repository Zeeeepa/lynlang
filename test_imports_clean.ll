; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [26 x i8] c"Testing clean imports...\0A\00", align 1
@str.1 = private unnamed_addr constant [12 x i8] c"abs(-42) = \00", align 1
@int_format = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.2 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.3 = private unnamed_addr constant [20 x i8] c"core.assert works!\0A\00", align 1
@str.4 = private unnamed_addr constant [33 x i8] c"\E2\9C\93 All imports work correctly!\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %x = alloca i32, align 4
  store i32 42, ptr %x, align 4
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %x2 = load i32, ptr %x, align 4
  %printf_int_call = call i32 (ptr, ...) @printf(ptr @int_format, i32 %x2)
  %printf_call3 = call i32 (ptr, ...) @printf(ptr @str.2)
  %printf_call4 = call i32 (ptr, ...) @printf(ptr @str.3)
  %printf_call5 = call i32 (ptr, ...) @printf(ptr @str.4)
  ret i32 0
}

declare i32 @printf(ptr, ...)
