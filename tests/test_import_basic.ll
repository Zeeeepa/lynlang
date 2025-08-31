; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [28 x i8] c"=== Basic Import Test ===\0A\0A\00", align 1
@str.1 = private unnamed_addr constant [22 x i8] c"Test 1: IO module... \00", align 1
@str.2 = private unnamed_addr constant [5 x i8] c"\E2\9C\93\0A\00", align 1
@str.3 = private unnamed_addr constant [24 x i8] c"Test 2: Math module... \00", align 1
@str.4 = private unnamed_addr constant [5 x i8] c"\E2\9C\93\0A\00", align 1
@str.5 = private unnamed_addr constant [24 x i8] c"Test 3: Core module... \00", align 1
@str.6 = private unnamed_addr constant [5 x i8] c"\E2\9C\93\0A\00", align 1
@str.7 = private unnamed_addr constant [31 x i8] c"\0A\E2\9C\85 All import tests passed!\0A\00", align 1
@str.8 = private unnamed_addr constant [47 x i8] c"Imports are at module level, not in comptime.\0A\00", align 1

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
  ret i32 0
}

declare i32 @printf(ptr, ...)
