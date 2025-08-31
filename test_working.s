	.text
	.file	"main"
	.globl	add                             # -- Begin function add
	.p2align	4, 0x90
	.type	add,@function
add:                                    # @add
	.cfi_startproc
# %bb.0:                                # %entry
                                        # kill: def $esi killed $esi def $rsi
                                        # kill: def $edi killed $edi def $rdi
	movl	%edi, -4(%rsp)
	movl	%esi, -8(%rsp)
	leal	(%rdi,%rsi), %eax
	retq
.Lfunc_end0:
	.size	add, .Lfunc_end0-add
	.cfi_endproc
                                        # -- End function
	.globl	main                            # -- Begin function main
	.p2align	4, 0x90
	.type	main,@function
main:                                   # @main
	.cfi_startproc
# %bb.0:                                # %entry
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	subq	$16, %rsp
	movl	$.Lstr, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$.Lstr.1, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$10, -12(%rbp)
	movl	$20, -8(%rbp)
	movl	$30, -4(%rbp)
	movb	$1, %al
	testb	%al, %al
	je	.LBB1_9
# %bb.1:                                # %match_0
	movl	$.Lstr.2, %edi
	jmp	.LBB1_2
.LBB1_9:                                # %test_1
	movl	$30, %eax
	cmpl	$30, %eax
	je	.LBB1_3
# %bb.10:                               # %match_1
	movl	$.Lstr.3, %edi
.LBB1_2:                                # %match_merge
	xorl	%eax, %eax
	callq	printf@PLT
.LBB1_3:                                # %match_merge
	movl	$.Lstr.4, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$15, %edi
	movl	$25, %esi
	callq	add@PLT
	movq	%rsp, %rcx
	leaq	-16(%rcx), %rsp
	movl	%eax, -16(%rcx)
	cmpl	$40, %eax
	jne	.LBB1_11
# %bb.4:                                # %match_013
	movl	$.Lstr.5, %edi
	jmp	.LBB1_5
.LBB1_11:                               # %test_114
	je	.LBB1_6
# %bb.12:                               # %match_117
	movl	$.Lstr.6, %edi
.LBB1_5:                                # %match_merge11
	xorl	%eax, %eax
	callq	printf@PLT
.LBB1_6:                                # %match_merge11
	movl	$.Lstr.7, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movq	%rsp, %rcx
	leaq	-16(%rcx), %rax
	movq	%rax, %rsp
	movl	$0, -16(%rcx)
	movq	%rsp, %rdx
	leaq	-16(%rdx), %rcx
	movq	%rcx, %rsp
	movl	$0, -16(%rdx)
	cmpl	$4, (%rcx)
	jg	.LBB1_23
	.p2align	4, 0x90
.LBB1_8:                                # %loop_body
                                        # =>This Inner Loop Header: Depth=1
	incl	(%rax)
	incl	(%rcx)
	cmpl	$4, (%rcx)
	jle	.LBB1_8
.LBB1_23:                               # %after_loop
	cmpl	$5, (%rax)
	jne	.LBB1_24
# %bb.13:                               # %match_031
	movl	$.Lstr.8, %edi
	jmp	.LBB1_14
.LBB1_24:                               # %test_132
	je	.LBB1_15
# %bb.25:                               # %match_135
	movl	$.Lstr.9, %edi
.LBB1_14:                               # %match_merge29
	xorl	%eax, %eax
	callq	printf@PLT
.LBB1_15:                               # %match_merge29
	movl	$.Lstr.10, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movq	%rsp, %rax
	leaq	-16(%rax), %rsp
	movabsq	$17179869187, %rcx              # imm = 0x400000003
	movq	%rcx, -16(%rax)
	movq	%rsp, %rax
	leaq	-16(%rax), %rsp
	movq	%rcx, -16(%rax)
	movq	%rsp, %rax
	leaq	-16(%rax), %rsp
	movl	$7, -16(%rax)
	movb	$1, %al
	testb	%al, %al
	je	.LBB1_26
# %bb.16:                               # %match_045
	movl	$.Lstr.11, %edi
	jmp	.LBB1_17
.LBB1_26:                               # %test_146
	movl	$7, %eax
	cmpl	$7, %eax
	je	.LBB1_18
# %bb.27:                               # %match_149
	movl	$.Lstr.12, %edi
.LBB1_17:                               # %match_merge43
	xorl	%eax, %eax
	callq	printf@PLT
.LBB1_18:                               # %match_merge43
	movl	$.Lstr.13, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movq	%rsp, %rax
	leaq	-16(%rax), %rsp
	movl	$42, -16(%rax)
	movl	$1, %eax
	movb	$1, %cl
	testb	%cl, %cl
	jne	.LBB1_19
# %bb.28:                               # %test_158
	xorl	%eax, %eax
.LBB1_19:                               # %match_merge55
	movq	%rsp, %rcx
	leaq	-16(%rcx), %rsp
	movl	%eax, -16(%rcx)
	testl	%eax, %eax
	je	.LBB1_29
# %bb.20:                               # %match_067
	movl	$.Lstr.14, %edi
	jmp	.LBB1_21
.LBB1_29:                               # %test_168
	jne	.LBB1_22
# %bb.30:                               # %match_171
	movl	$.Lstr.15, %edi
.LBB1_21:                               # %match_merge65
	xorl	%eax, %eax
	callq	printf@PLT
.LBB1_22:                               # %match_merge65
	movl	$.Lstr.16, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	xorl	%eax, %eax
	movq	%rbp, %rsp
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end1:
	.size	main, .Lfunc_end1-main
	.cfi_endproc
                                        # -- End function
	.type	.Lstr,@object                   # @str
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lstr:
	.asciz	"=== Zen Working Features Test ===\n\n"
	.size	.Lstr, 36

	.type	.Lstr.1,@object                 # @str.1
.Lstr.1:
	.asciz	"1. Variables and arithmetic: "
	.size	.Lstr.1, 30

	.type	.Lstr.2,@object                 # @str.2
.Lstr.2:
	.asciz	"PASS\n"
	.size	.Lstr.2, 6

	.type	.Lstr.3,@object                 # @str.3
.Lstr.3:
	.asciz	"FAIL\n"
	.size	.Lstr.3, 6

	.type	.Lstr.4,@object                 # @str.4
.Lstr.4:
	.asciz	"2. Function calls: "
	.size	.Lstr.4, 20

	.type	.Lstr.5,@object                 # @str.5
.Lstr.5:
	.asciz	"PASS\n"
	.size	.Lstr.5, 6

	.type	.Lstr.6,@object                 # @str.6
.Lstr.6:
	.asciz	"FAIL\n"
	.size	.Lstr.6, 6

	.type	.Lstr.7,@object                 # @str.7
.Lstr.7:
	.asciz	"3. Loops: "
	.size	.Lstr.7, 11

	.type	.Lstr.8,@object                 # @str.8
.Lstr.8:
	.asciz	"PASS\n"
	.size	.Lstr.8, 6

	.type	.Lstr.9,@object                 # @str.9
.Lstr.9:
	.asciz	"FAIL\n"
	.size	.Lstr.9, 6

	.type	.Lstr.10,@object                # @str.10
.Lstr.10:
	.asciz	"4. Structs: "
	.size	.Lstr.10, 13

	.type	.Lstr.11,@object                # @str.11
.Lstr.11:
	.asciz	"PASS\n"
	.size	.Lstr.11, 6

	.type	.Lstr.12,@object                # @str.12
.Lstr.12:
	.asciz	"FAIL\n"
	.size	.Lstr.12, 6

	.type	.Lstr.13,@object                # @str.13
.Lstr.13:
	.asciz	"5. Pattern matching: "
	.size	.Lstr.13, 22

	.type	.Lstr.14,@object                # @str.14
.Lstr.14:
	.asciz	"PASS\n"
	.size	.Lstr.14, 6

	.type	.Lstr.15,@object                # @str.15
.Lstr.15:
	.asciz	"FAIL\n"
	.size	.Lstr.15, 6

	.type	.Lstr.16,@object                # @str.16
.Lstr.16:
	.asciz	"\n=== All working features tested ===\n"
	.size	.Lstr.16, 38

	.section	".note.GNU-stack","",@progbits
