; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [13 x i8] c"Hello, Zen!\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  ret i32 0
}

declare i32 @printf(ptr, ...)
