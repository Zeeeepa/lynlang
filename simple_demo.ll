; ModuleID = 'main'
source_filename = "main"

@str = private unnamed_addr constant [17 x i8] c"Welcome to Zen!\0A\00", align 1
@str.1 = private unnamed_addr constant [18 x i8] c"===============\0A\0A\00", align 1
@str.2 = private unnamed_addr constant [26 x i8] c"Testing math operations:\0A\00", align 1
@str.3 = private unnamed_addr constant [5 x i8] c"x = \00", align 1
@int_format = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.4 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.5 = private unnamed_addr constant [5 x i8] c"y = \00", align 1
@int_format.6 = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.7 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.8 = private unnamed_addr constant [10 x i8] c"abs(x) = \00", align 1
@int_format.9 = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.10 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.11 = private unnamed_addr constant [10 x i8] c"abs(y) = \00", align 1
@int_format.12 = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.13 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.14 = private unnamed_addr constant [19 x i8] c"abs(x) + abs(y) = \00", align 1
@int_format.15 = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@str.16 = private unnamed_addr constant [2 x i8] c"\0A\00", align 1
@str.17 = private unnamed_addr constant [8 x i8] c"\0ADone!\0A\00", align 1

define i32 @calculate(i32 %a, i32 %b) {
entry:
  %a1 = alloca i32, align 4
  store i32 %a, ptr %a1, align 4
  %b2 = alloca i32, align 4
  store i32 %b, ptr %b2, align 4
  %a3 = load i32, ptr %a1, align 4
  %is_negative = icmp slt i32 %a3, 0
  %neg = sub i32 0, %a3
  %abs = select i1 %is_negative, i32 %neg, i32 %a3
  %abs_a = alloca i32, align 4
  store i32 %abs, ptr %abs_a, align 4
  %b4 = load i32, ptr %b2, align 4
  %is_negative5 = icmp slt i32 %b4, 0
  %neg6 = sub i32 0, %b4
  %abs7 = select i1 %is_negative5, i32 %neg6, i32 %b4
  %abs_b = alloca i32, align 4
  store i32 %abs7, ptr %abs_b, align 4
  %abs_a8 = load i32, ptr %abs_a, align 4
  %abs_b9 = load i32, ptr %abs_b, align 4
  %addtmp = add i32 %abs_a8, %abs_b9
  ret i32 %addtmp
}

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @str)
  %printf_call1 = call i32 (ptr, ...) @printf(ptr @str.1)
  %printf_call2 = call i32 (ptr, ...) @printf(ptr @str.2)
  %x = alloca i32, align 4
  store i32 -42, ptr %x, align 4
  %y = alloca i32, align 4
  store i32 17, ptr %y, align 4
  %printf_call3 = call i32 (ptr, ...) @printf(ptr @str.3)
  %x4 = load i32, ptr %x, align 4
  %printf_int_call = call i32 (ptr, ...) @printf(ptr @int_format, i32 %x4)
  %printf_call5 = call i32 (ptr, ...) @printf(ptr @str.4)
  %printf_call6 = call i32 (ptr, ...) @printf(ptr @str.5)
  %y7 = load i32, ptr %y, align 4
  %printf_int_call8 = call i32 (ptr, ...) @printf(ptr @int_format.6, i32 %y7)
  %printf_call9 = call i32 (ptr, ...) @printf(ptr @str.7)
  %printf_call10 = call i32 (ptr, ...) @printf(ptr @str.8)
  %x11 = load i32, ptr %x, align 4
  %is_negative = icmp slt i32 %x11, 0
  %neg = sub i32 0, %x11
  %abs = select i1 %is_negative, i32 %neg, i32 %x11
  %printf_int_call12 = call i32 (ptr, ...) @printf(ptr @int_format.9, i32 %abs)
  %printf_call13 = call i32 (ptr, ...) @printf(ptr @str.10)
  %printf_call14 = call i32 (ptr, ...) @printf(ptr @str.11)
  %y15 = load i32, ptr %y, align 4
  %is_negative16 = icmp slt i32 %y15, 0
  %neg17 = sub i32 0, %y15
  %abs18 = select i1 %is_negative16, i32 %neg17, i32 %y15
  %printf_int_call19 = call i32 (ptr, ...) @printf(ptr @int_format.12, i32 %abs18)
  %printf_call20 = call i32 (ptr, ...) @printf(ptr @str.13)
  %x21 = load i32, ptr %x, align 4
  %y22 = load i32, ptr %y, align 4
  %calltmp = call i32 @calculate(i32 %x21, i32 %y22)
  %result = alloca i32, align 4
  store i32 %calltmp, ptr %result, align 4
  %printf_call23 = call i32 (ptr, ...) @printf(ptr @str.14)
  %result24 = load i32, ptr %result, align 4
  %printf_int_call25 = call i32 (ptr, ...) @printf(ptr @int_format.15, i32 %result24)
  %printf_call26 = call i32 (ptr, ...) @printf(ptr @str.16)
  %printf_call27 = call i32 (ptr, ...) @printf(ptr @str.17)
  ret i32 0
}

declare i32 @printf(ptr, ...)
