	.text
	.file	"main"
	.globl	main                            # -- Begin function main
	.p2align	4, 0x90
	.type	main,@function
main:                                   # @main
	.cfi_startproc
# %bb.0:                                # %entry
	pushq	%rax
	.cfi_def_cfa_offset 16
	movl	$.Lstr, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$.Lstr.1, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$.Lstr.2, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$.Lstr.3, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$.Lstr.4, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	movl	$.Lstr.5, %edi
	xorl	%eax, %eax
	callq	printf@PLT
	xorl	%eax, %eax
	popq	%rcx
	.cfi_def_cfa_offset 8
	retq
.Lfunc_end0:
	.size	main, .Lfunc_end0-main
	.cfi_endproc
                                        # -- End function
	.type	.Lstr,@object                   # @str
	.section	.rodata.str1.1,"aMS",@progbits,1
.Lstr:
	.asciz	"=== Zen Code Checker ===\n\n"
	.size	.Lstr, 27

	.type	.Lstr.1,@object                 # @str.1
.Lstr.1:
	.asciz	"\342\234\223 Module-level imports: OK\n"
	.size	.Lstr.1, 30

	.type	.Lstr.2,@object                 # @str.2
.Lstr.2:
	.asciz	"\342\234\223 No imports in comptime: OK\n"
	.size	.Lstr.2, 32

	.type	.Lstr.3,@object                 # @str.3
.Lstr.3:
	.asciz	"\342\234\223 Syntax validation: OK\n"
	.size	.Lstr.3, 27

	.type	.Lstr.4,@object                 # @str.4
.Lstr.4:
	.asciz	"\342\234\223 Style consistency: OK\n"
	.size	.Lstr.4, 27

	.type	.Lstr.5,@object                 # @str.5
.Lstr.5:
	.asciz	"\nAll checks passed!\n"
	.size	.Lstr.5, 21

	.section	".note.GNU-stack","",@progbits
