; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [30 x i8] c"Testing nested conditionals:\0A\00", align 1
@str.1 = private unnamed_addr constant [26 x i8] c"(0,0) -> %d (expected 0)\0A\00", align 1
@str.2 = private unnamed_addr constant [26 x i8] c"(0,1) -> %d (expected 1)\0A\00", align 1
@str.3 = private unnamed_addr constant [26 x i8] c"(1,0) -> %d (expected 2)\0A\00", align 1
@str.4 = private unnamed_addr constant [26 x i8] c"(1,1) -> %d (expected 3)\0A\00", align 1

declare i32 @printf(ptr, ...)

define i32 @test_nested(i32 %x, i32 %y) {
entry:
  %x1 = alloca i32, align 4
  store i32 %x, ptr %x1, align 4
  %y2 = alloca i32, align 4
  store i32 %y, ptr %y2, align 4
  %x3 = load i32, ptr %x1, align 4
  %eqtmp = icmp eq i32 %x3, 0
  %int_eq = icmp eq i1 %eqtmp, true
  br i1 %int_eq, label %match_0, label %pattern_unmatched

match_merge:                                      ; preds = %pattern_unmatched, %match_merge6
  %match_result21 = phi i32 [ %match_result20, %match_merge6 ], [ 0, %pattern_unmatched ]
  ret i32 %match_result21

match_0:                                          ; preds = %entry
  %y4 = load i32, ptr %y2, align 4
  %eqtmp5 = icmp eq i32 %y4, 0
  %int_eq7 = icmp eq i1 %eqtmp5, true
  br i1 %int_eq7, label %match_08, label %test_1

pattern_unmatched:                                ; preds = %entry
  br label %match_merge

match_merge6:                                     ; preds = %pattern_unmatched11, %match_merge14, %match_1, %match_08
  %match_result20 = phi i32 [ 0, %match_08 ], [ 1, %match_1 ], [ %match_result, %match_merge14 ], [ 0, %pattern_unmatched11 ]
  br label %match_merge

match_08:                                         ; preds = %match_0
  br label %match_merge6

test_1:                                           ; preds = %match_0
  %int_eq9 = icmp eq i1 %eqtmp5, false
  br i1 %int_eq9, label %match_1, label %test_2

match_1:                                          ; preds = %test_1
  br label %match_merge6

test_2:                                           ; preds = %test_1
  %int_eq10 = icmp eq i1 %eqtmp5, false
  br i1 %int_eq10, label %match_2, label %pattern_unmatched11

match_2:                                          ; preds = %test_2
  %y12 = load i32, ptr %y2, align 4
  %eqtmp13 = icmp eq i32 %y12, 0
  %int_eq15 = icmp eq i1 %eqtmp13, true
  br i1 %int_eq15, label %match_016, label %test_117

pattern_unmatched11:                              ; preds = %test_2
  br label %match_merge6

match_merge14:                                    ; preds = %match_119, %match_016
  %match_result = phi i32 [ 2, %match_016 ], [ 3, %match_119 ]
  br label %match_merge6

match_016:                                        ; preds = %match_2
  br label %match_merge14

test_117:                                         ; preds = %match_2
  %int_eq18 = icmp eq i1 %eqtmp13, false
  br i1 %int_eq18, label %match_119, label %match_119

match_119:                                        ; preds = %test_117, %test_117
  br label %match_merge14
}

define i32 @main() {
entry:
  %calltmp = call i32 (ptr, ...) @printf(ptr @str)
  %calltmp1 = call i32 @test_nested(i32 0, i32 0)
  %calltmp2 = call i32 (ptr, ...) @printf(ptr @str.1, i32 %calltmp1)
  %calltmp3 = call i32 @test_nested(i32 0, i32 1)
  %calltmp4 = call i32 (ptr, ...) @printf(ptr @str.2, i32 %calltmp3)
  %calltmp5 = call i32 @test_nested(i32 1, i32 0)
  %calltmp6 = call i32 (ptr, ...) @printf(ptr @str.3, i32 %calltmp5)
  %calltmp7 = call i32 @test_nested(i32 1, i32 1)
  %calltmp8 = call i32 (ptr, ...) @printf(ptr @str.4, i32 %calltmp7)
  ret i32 0
}

