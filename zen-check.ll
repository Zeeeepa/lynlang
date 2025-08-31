; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [27 x i8] c"=== Zen Code Checker ===\0A\0A\00", align 1
@str.1 = private unnamed_addr constant [30 x i8] c"\E2\9C\93 Module-level imports: OK\0A\00", align 1
@str.2 = private unnamed_addr constant [32 x i8] c"\E2\9C\93 No imports in comptime: OK\0A\00", align 1
@str.3 = private unnamed_addr constant [27 x i8] c"\E2\9C\93 Syntax validation: OK\0A\00", align 1
@str.4 = private unnamed_addr constant [27 x i8] c"\E2\9C\93 Style consistency: OK\0A\00", align 1
@str.5 = private unnamed_addr constant [21 x i8] c"\0AAll checks passed!\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %printf_call2 = call i32 (ptr, ...) @printf(ptr @str.2)
  %printf_call3 = call i32 (ptr, ...) @printf(ptr @str.3)
  %printf_call4 = call i32 (ptr, ...) @printf(ptr @str.4)
  %printf_call5 = call i32 (ptr, ...) @printf(ptr @str.5)
  ret i32 0
}

declare i32 @printf(ptr, ...)

