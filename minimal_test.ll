; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [25 x i8] c"Testing Zen compiler...\0A\00", align 1
@str.1 = private unnamed_addr constant [20 x i8] c"fibonacci(10) = %d\0A\00", align 1
@str.2 = private unnamed_addr constant [14 x i8] c"Expected: 55\0A\00", align 1

declare i32 @printf(ptr, ...)

define i32 @fibonacci(i32 %n) {
entry:
  %n1 = alloca i32, align 4
  store i32 %n, ptr %n1, align 4
  %n2 = load i32, ptr %n1, align 4
  %letmp = icmp sle i32 %n2, 1
  %int_eq = icmp eq i1 %letmp, true
  br i1 %int_eq, label %match_1_0, label %test_1_1

match_merge_1:                                    ; preds = %pattern_unmatched_1, %match_1_1, %match_1_0
  %match_result = phi i32 [ %n3, %match_1_0 ], [ %addtmp, %match_1_1 ], [ 0, %pattern_unmatched_1 ]
  %n9 = load i32, ptr %n1, align 4
  %letmp10 = icmp sle i32 %n9, 1
  %int_eq11 = icmp eq i1 %letmp10, true
  br i1 %int_eq11, label %match_2_0, label %test_2_1

match_1_0:                                        ; preds = %entry
  %n3 = load i32, ptr %n1, align 4
  br label %match_merge_1

test_1_1:                                         ; preds = %entry
  %int_eq4 = icmp eq i1 %letmp, false
  br i1 %int_eq4, label %match_1_1, label %pattern_unmatched_1

match_1_1:                                        ; preds = %test_1_1
  %n5 = load i32, ptr %n1, align 4
  %subtmp = sub i32 %n5, 1
  %calltmp = call i32 @fibonacci(i32 %subtmp)
  %n6 = load i32, ptr %n1, align 4
  %subtmp7 = sub i32 %n6, 2
  %calltmp8 = call i32 @fibonacci(i32 %subtmp7)
  %addtmp = add i32 %calltmp, %calltmp8
  br label %match_merge_1

pattern_unmatched_1:                              ; preds = %test_1_1
  br label %match_merge_1

match_merge_2:                                    ; preds = %pattern_unmatched_2, %match_2_1, %match_2_0
  %match_result21 = phi i32 [ %n12, %match_2_0 ], [ %addtmp20, %match_2_1 ], [ 0, %pattern_unmatched_2 ]
  ret i32 %match_result21

match_2_0:                                        ; preds = %match_merge_1
  %n12 = load i32, ptr %n1, align 4
  br label %match_merge_2

test_2_1:                                         ; preds = %match_merge_1
  %int_eq13 = icmp eq i1 %letmp10, false
  br i1 %int_eq13, label %match_2_1, label %pattern_unmatched_2

match_2_1:                                        ; preds = %test_2_1
  %n14 = load i32, ptr %n1, align 4
  %subtmp15 = sub i32 %n14, 1
  %calltmp16 = call i32 @fibonacci(i32 %subtmp15)
  %n17 = load i32, ptr %n1, align 4
  %subtmp18 = sub i32 %n17, 2
  %calltmp19 = call i32 @fibonacci(i32 %subtmp18)
  %addtmp20 = add i32 %calltmp16, %calltmp19
  br label %match_merge_2

pattern_unmatched_2:                              ; preds = %test_2_1
  br label %match_merge_2
}

define i32 @main() {
entry:
  %calltmp = call i32 (ptr, ...) @printf(ptr @str)
  %calltmp1 = call i32 @fibonacci(i32 10)
  %result = alloca i32, align 4
  store i32 %calltmp1, ptr %result, align 4
  %result2 = load i32, ptr %result, align 4
  %calltmp3 = call i32 (ptr, ...) @printf(ptr @str.1, i32 %result2)
  %calltmp4 = call i32 (ptr, ...) @printf(ptr @str.2)
  ret i32 0
}
