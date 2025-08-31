; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [24 x i8] c"Hello from Zen binary!\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  ret i32 0
}

declare i32 @printf(ptr, ...)
