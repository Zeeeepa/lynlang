; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [34 x i8] c"test_nested(0) = %d (expect 100)\0A\00", align 1
@str.1 = private unnamed_addr constant [34 x i8] c"test_nested(1) = %d (expect 200)\0A\00", align 1
@str.2 = private unnamed_addr constant [34 x i8] c"test_nested(2) = %d (expect 300)\0A\00", align 1

declare i32 @printf(ptr, ...)

define i32 @test_nested(i32 %x) {
entry:
  %x1 = alloca i32, align 4
  store i32 %x, ptr %x1, align 4
  %x2 = load i32, ptr %x1, align 4
  %eqtmp = icmp eq i32 %x2, 0
  %int_eq = icmp eq i1 %eqtmp, true
  br i1 %int_eq, label %match_0, label %test_1

match_merge:                                      ; preds = %match_merge6, %match_0
  %match_result13 = phi i32 [ 100, %match_0 ], [ %match_result, %match_merge6 ]
  ret i32 %match_result13

match_0:                                          ; preds = %entry
  br label %match_merge

test_1:                                           ; preds = %entry
  %int_eq3 = icmp eq i1 %eqtmp, false
  br i1 %int_eq3, label %match_1, label %pattern_unmatched

match_1:                                          ; preds = %test_1
  %x4 = load i32, ptr %x1, align 4
  %eqtmp5 = icmp eq i32 %x4, 1
  %int_eq7 = icmp eq i1 %eqtmp5, true
  br i1 %int_eq7, label %match_08, label %test_19

pattern_unmatched:                                ; preds = %test_1
  unreachable

match_merge6:                                     ; preds = %match_111, %match_08
  %match_result = phi i32 [ 200, %match_08 ], [ 300, %match_111 ]
  br label %match_merge

match_08:                                         ; preds = %match_1
  br label %match_merge6

test_19:                                          ; preds = %match_1
  %int_eq10 = icmp eq i1 %eqtmp5, false
  br i1 %int_eq10, label %match_111, label %pattern_unmatched12

match_111:                                        ; preds = %test_19
  br label %match_merge6

pattern_unmatched12:                              ; preds = %test_19
  unreachable
}

define i32 @main() {
entry:
  %calltmp = call i32 @test_nested(i32 0)
  %calltmp1 = call i32 (ptr, ...) @printf(ptr @str, i32 %calltmp)
  %calltmp2 = call i32 @test_nested(i32 1)
  %calltmp3 = call i32 (ptr, ...) @printf(ptr @str.1, i32 %calltmp2)
  %calltmp4 = call i32 @test_nested(i32 2)
  %calltmp5 = call i32 (ptr, ...) @printf(ptr @str.2, i32 %calltmp4)
  ret i32 0
}

