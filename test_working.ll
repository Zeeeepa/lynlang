; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [36 x i8] c"=== Zen Working Features Test ===\0A\0A\00", align 1
@str.1 = private unnamed_addr constant [30 x i8] c"1. Variables and arithmetic: \00", align 1
@str.2 = private unnamed_addr constant [6 x i8] c"PASS\0A\00", align 1
@str.3 = private unnamed_addr constant [6 x i8] c"FAIL\0A\00", align 1
@str.4 = private unnamed_addr constant [20 x i8] c"2. Function calls: \00", align 1
@str.5 = private unnamed_addr constant [6 x i8] c"PASS\0A\00", align 1
@str.6 = private unnamed_addr constant [6 x i8] c"FAIL\0A\00", align 1
@str.7 = private unnamed_addr constant [11 x i8] c"3. Loops: \00", align 1
@str.8 = private unnamed_addr constant [6 x i8] c"PASS\0A\00", align 1
@str.9 = private unnamed_addr constant [6 x i8] c"FAIL\0A\00", align 1
@str.10 = private unnamed_addr constant [13 x i8] c"4. Structs: \00", align 1
@str.11 = private unnamed_addr constant [6 x i8] c"PASS\0A\00", align 1
@str.12 = private unnamed_addr constant [6 x i8] c"FAIL\0A\00", align 1
@str.13 = private unnamed_addr constant [22 x i8] c"5. Pattern matching: \00", align 1
@str.14 = private unnamed_addr constant [6 x i8] c"PASS\0A\00", align 1
@str.15 = private unnamed_addr constant [6 x i8] c"FAIL\0A\00", align 1
@str.16 = private unnamed_addr constant [38 x i8] c"\0A=== All working features tested ===\0A\00", align 1

define i32 @add(i32 %a, i32 %b) {
entry:
  %a1 = alloca i32, align 4
  store i32 %a, ptr %a1, align 4
  %b2 = alloca i32, align 4
  store i32 %b, ptr %b2, align 4
  %a3 = load i32, ptr %a1, align 4
  %b4 = load i32, ptr %b2, align 4
  %addtmp = add i32 %a3, %b4
  ret i32 %addtmp
}

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %x = alloca i32, align 4
  store i32 10, ptr %x, align 4
  %y = alloca i32, align 4
  store i32 20, ptr %y, align 4
  %x2 = load i32, ptr %x, align 4
  %y3 = load i32, ptr %y, align 4
  %addtmp = add i32 %x2, %y3
  %z = alloca i32, align 4
  store i32 %addtmp, ptr %z, align 4
  %z4 = load i32, ptr %z, align 4
  %eqtmp = icmp eq i32 %z4, 30
  %int_eq = icmp eq i1 %eqtmp, true
  br i1 %int_eq, label %match_0, label %test_1

match_merge:                                      ; preds = %pattern_unmatched, %match_1, %match_0
  %match_result = phi i32 [ 0, %match_0 ], [ 0, %match_1 ], [ 0, %pattern_unmatched ]
  %printf_call8 = call i32 (ptr, ...) @printf(ptr @str.4)
  %calltmp = call i32 @add(i32 15, i32 25)
  %result = alloca i32, align 4
  store i32 %calltmp, ptr %result, align 4
  %result9 = load i32, ptr %result, align 4
  %eqtmp10 = icmp eq i32 %result9, 40
  %int_eq12 = icmp eq i1 %eqtmp10, true
  br i1 %int_eq12, label %match_013, label %test_114

match_0:                                          ; preds = %entry
  %printf_call5 = call i32 (ptr, ...) @printf(ptr @str.2)
  br label %match_merge

test_1:                                           ; preds = %entry
  %int_eq6 = icmp eq i1 %eqtmp, false
  br i1 %int_eq6, label %match_1, label %pattern_unmatched

match_1:                                          ; preds = %test_1
  %printf_call7 = call i32 (ptr, ...) @printf(ptr @str.3)
  br label %match_merge

pattern_unmatched:                                ; preds = %test_1
  br label %match_merge

match_merge11:                                    ; preds = %pattern_unmatched18, %match_117, %match_013
  %match_result20 = phi i32 [ 0, %match_013 ], [ 0, %match_117 ], [ 0, %pattern_unmatched18 ]
  %printf_call21 = call i32 (ptr, ...) @printf(ptr @str.7)
  %count = alloca i32, align 4
  store i32 0, ptr %count, align 4
  %i = alloca i32, align 4
  store i32 0, ptr %i, align 4
  br label %loop_header

match_013:                                        ; preds = %match_merge
  %printf_call15 = call i32 (ptr, ...) @printf(ptr @str.5)
  br label %match_merge11

test_114:                                         ; preds = %match_merge
  %int_eq16 = icmp eq i1 %eqtmp10, false
  br i1 %int_eq16, label %match_117, label %pattern_unmatched18

match_117:                                        ; preds = %test_114
  %printf_call19 = call i32 (ptr, ...) @printf(ptr @str.6)
  br label %match_merge11

pattern_unmatched18:                              ; preds = %test_114
  br label %match_merge11

loop_header:                                      ; preds = %loop_body, %match_merge11
  %i22 = load i32, ptr %i, align 4
  %lttmp = icmp slt i32 %i22, 5
  %zext_lt = zext i1 %lttmp to i64
  %loop_condition = icmp ne i64 %zext_lt, 0
  br i1 %loop_condition, label %loop_body, label %after_loop

loop_body:                                        ; preds = %loop_header
  %count23 = load i32, ptr %count, align 4
  %addtmp24 = add i32 %count23, 1
  store i32 %addtmp24, ptr %count, align 4
  %i25 = load i32, ptr %i, align 4
  %addtmp26 = add i32 %i25, 1
  store i32 %addtmp26, ptr %i, align 4
  br label %loop_header

after_loop:                                       ; preds = %loop_header
  %count27 = load i32, ptr %count, align 4
  %eqtmp28 = icmp eq i32 %count27, 5
  %int_eq30 = icmp eq i1 %eqtmp28, true
  br i1 %int_eq30, label %match_031, label %test_132

match_merge29:                                    ; preds = %pattern_unmatched36, %match_135, %match_031
  %match_result38 = phi i32 [ 0, %match_031 ], [ 0, %match_135 ], [ 0, %pattern_unmatched36 ]
  %printf_call39 = call i32 (ptr, ...) @printf(ptr @str.10)
  %Point_tmp = alloca { i32, i32 }, align 8
  %x_ptr = getelementptr inbounds { i32, i32 }, ptr %Point_tmp, i32 0, i32 0
  store i32 3, ptr %x_ptr, align 4
  %y_ptr = getelementptr inbounds { i32, i32 }, ptr %Point_tmp, i32 0, i32 1
  store i32 4, ptr %y_ptr, align 4
  %Point_val = load { i32, i32 }, ptr %Point_tmp, align 4
  %p = alloca i64, align 8
  store { i32, i32 } %Point_val, ptr %p, align 4
  %p.x = getelementptr { i32, i32 }, ptr %p, i32 0, i32 0
  %load_x = load i32, ptr %p.x, align 4
  %p.y = getelementptr { i32, i32 }, ptr %p, i32 0, i32 1
  %load_y = load i32, ptr %p.y, align 4
  %addtmp40 = add i32 %load_x, %load_y
  %sum = alloca i32, align 4
  store i32 %addtmp40, ptr %sum, align 4
  %sum41 = load i32, ptr %sum, align 4
  %eqtmp42 = icmp eq i32 %sum41, 7
  %int_eq44 = icmp eq i1 %eqtmp42, true
  br i1 %int_eq44, label %match_045, label %test_146

match_031:                                        ; preds = %after_loop
  %printf_call33 = call i32 (ptr, ...) @printf(ptr @str.8)
  br label %match_merge29

test_132:                                         ; preds = %after_loop
  %int_eq34 = icmp eq i1 %eqtmp28, false
  br i1 %int_eq34, label %match_135, label %pattern_unmatched36

match_135:                                        ; preds = %test_132
  %printf_call37 = call i32 (ptr, ...) @printf(ptr @str.9)
  br label %match_merge29

pattern_unmatched36:                              ; preds = %test_132
  br label %match_merge29

match_merge43:                                    ; preds = %pattern_unmatched50, %match_149, %match_045
  %match_result52 = phi i32 [ 0, %match_045 ], [ 0, %match_149 ], [ 0, %pattern_unmatched50 ]
  %printf_call53 = call i32 (ptr, ...) @printf(ptr @str.13)
  %value = alloca i32, align 4
  store i32 42, ptr %value, align 4
  %value54 = load i32, ptr %value, align 4
  %gttmp = icmp sgt i32 %value54, 40
  %int_eq56 = icmp eq i1 %gttmp, true
  br i1 %int_eq56, label %match_057, label %test_158

match_045:                                        ; preds = %match_merge29
  %printf_call47 = call i32 (ptr, ...) @printf(ptr @str.11)
  br label %match_merge43

test_146:                                         ; preds = %match_merge29
  %int_eq48 = icmp eq i1 %eqtmp42, false
  br i1 %int_eq48, label %match_149, label %pattern_unmatched50

match_149:                                        ; preds = %test_146
  %printf_call51 = call i32 (ptr, ...) @printf(ptr @str.12)
  br label %match_merge43

pattern_unmatched50:                              ; preds = %test_146
  br label %match_merge43

match_merge55:                                    ; preds = %pattern_unmatched61, %match_160, %match_057
  %match_result62 = phi i32 [ 1, %match_057 ], [ 0, %match_160 ], [ 0, %pattern_unmatched61 ]
  %category = alloca i32, align 4
  store i32 %match_result62, ptr %category, align 4
  %category63 = load i32, ptr %category, align 4
  %eqtmp64 = icmp eq i32 %category63, 1
  %int_eq66 = icmp eq i1 %eqtmp64, true
  br i1 %int_eq66, label %match_067, label %test_168

match_057:                                        ; preds = %match_merge43
  br label %match_merge55

test_158:                                         ; preds = %match_merge43
  %int_eq59 = icmp eq i1 %gttmp, false
  br i1 %int_eq59, label %match_160, label %pattern_unmatched61

match_160:                                        ; preds = %test_158
  br label %match_merge55

pattern_unmatched61:                              ; preds = %test_158
  br label %match_merge55

match_merge65:                                    ; preds = %pattern_unmatched72, %match_171, %match_067
  %match_result74 = phi i32 [ 0, %match_067 ], [ 0, %match_171 ], [ 0, %pattern_unmatched72 ]
  %printf_call75 = call i32 (ptr, ...) @printf(ptr @str.16)
  ret i32 0

match_067:                                        ; preds = %match_merge55
  %printf_call69 = call i32 (ptr, ...) @printf(ptr @str.14)
  br label %match_merge65

test_168:                                         ; preds = %match_merge55
  %int_eq70 = icmp eq i1 %eqtmp64, false
  br i1 %int_eq70, label %match_171, label %pattern_unmatched72

match_171:                                        ; preds = %test_168
  %printf_call73 = call i32 (ptr, ...) @printf(ptr @str.15)
  br label %match_merge65

pattern_unmatched72:                              ; preds = %test_168
  br label %match_merge65
}

declare i32 @printf(ptr, ...)

