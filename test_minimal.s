	.text
	.file	"main"
	.globl	main                            # -- Begin function main
	.p2align	4, 0x90
	.type	main,@function
main:                                   # @main
	.cfi_startproc
# %bb.0:                                # %entry
	subq	$24, %rsp
	.cfi_def_cfa_offset 32
	movl	$.Lstr, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$10, 20(%rsp)
	movl	$20, 16(%rsp)
	movl	$30, 12(%rsp)
	movb	$1, %al
	testb	%al, %al
	je	.LBB0_4
# %bb.1:                                # %match_0
	movl	$.Lstr.1, %edi
	jmp	.LBB0_2
.LBB0_4:                                # %test_1
	movl	$30, %eax
	cmpl	$30, %eax
	je	.LBB0_3
# %bb.5:                                # %match_1
	movl	$.Lstr.2, %edi
.LBB0_2:                                # %match_merge
	xorl	%eax, %eax
	callq	printf@PLT
.LBB0_3:                                # %match_merge
	xorl	%eax, %eax
	addq	$24, %rsp
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end0:
	.size	main, .Lfunc_end0-main
	.cfi_endproc
                                        # -- End function
	.type	.Lstr,@object                   # @str
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lstr:
	.asciz	"Zen Bootstrap Test: "
	.size	.Lstr, 21

	.type	.Lstr.1,@object                 # @str.1
.Lstr.1:
	.asciz	"PASS\n"
	.size	.Lstr.1, 6

	.type	.Lstr.2,@object                 # @str.2
.Lstr.2:
	.asciz	"FAIL\n"
	.size	.Lstr.2, 6

	.section	".note.GNU-stack","",@progbits
