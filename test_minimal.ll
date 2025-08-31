; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [21 x i8] c"Zen Bootstrap Test: \00", align 1
@str.1 = private unnamed_addr constant [6 x i8] c"PASS\0A\00", align 1
@str.2 = private unnamed_addr constant [6 x i8] c"FAIL\0A\00", align 1

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %x = alloca i32, align 4
  store i32 10, ptr %x, align 4
  %y = alloca i32, align 4
  store i32 20, ptr %y, align 4
  %x1 = load i32, ptr %x, align 4
  %y2 = load i32, ptr %y, align 4
  %addtmp = add i32 %x1, %y2
  %z = alloca i32, align 4
  store i32 %addtmp, ptr %z, align 4
  %z3 = load i32, ptr %z, align 4
  %eqtmp = icmp eq i32 %z3, 30
  %int_eq = icmp eq i1 %eqtmp, true
  br i1 %int_eq, label %match_0, label %test_1

match_merge:                                      ; preds = %pattern_unmatched, %match_1, %match_0
  %match_result = phi i32 [ 0, %match_0 ], [ 0, %match_1 ], [ 0, %pattern_unmatched ]
  ret i32 0

match_0:                                          ; preds = %entry
  %printf_call4 = call i32 (ptr, ...) @printf(ptr @str.1)
  br label %match_merge

test_1:                                           ; preds = %entry
  %int_eq5 = icmp eq i1 %eqtmp, false
  br i1 %int_eq5, label %match_1, label %pattern_unmatched

match_1:                                          ; preds = %test_1
  %printf_call6 = call i32 (ptr, ...) @printf(ptr @str.2)
  br label %match_merge

pattern_unmatched:                                ; preds = %test_1
  br label %match_merge
}

declare i32 @printf(ptr, ...)

