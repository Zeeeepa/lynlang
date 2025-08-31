; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [6 x i8] c"Test\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %x = alloca i64, align 8
  store i64 42, ptr %x, align 4
  ret i32 0
}

declare i32 @printf(ptr, ...)
