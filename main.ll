; ModuleID = 'main'
source_filename = "main"

define i32 @main() {
entry:
  %x = alloca i32, align 4
  store i32 42, ptr %x, align 4
  %count = alloca i32, align 4
  store i32 0, ptr %count, align 4
  %count1 = load i32, ptr %count, align 4
  %addtmp = add i32 %count1, 1
  store i32 %addtmp, ptr %count, align 4
  %x2 = load i32, ptr %x, align 4
  %count3 = load i32, ptr %count, align 4
  %addtmp4 = add i32 %x2, %count3
  %result = alloca i32, align 4
  %x5 = load i32, ptr %x, align 4
  %count6 = load i32, ptr %count, align 4
  %addtmp7 = add i32 %x5, %count6
  store i32 %addtmp7, ptr %result, align 4
  %result8 = load i32, ptr %result, align 4
  ret i32 %result8
}

