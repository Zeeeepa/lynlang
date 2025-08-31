; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [17 x i8] c"Hello from Zen!\0A\00", align 1

declare i32 @printf(ptr, ...)

define i32 @main() {
entry:
  %calltmp = call i32 (ptr, ...) @printf(ptr @str)
  ret i32 0
}
