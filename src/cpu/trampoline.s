/*
 * This file implements the trampoline code used to boot CPU cores for multicore processing.
 * The below code is real mode assembly, thus before jumping to the real kernel code, it is
 * required to switch to protected mode.
 *
 * The code will be relocated at an address at which it can be accessed in real mode.
 */

# The offset in physical memory at which the trampoline is located
.set TRAMPOLINE_OFFSET, 0x8000

.global cpu_trampoline
.global trampoline_stacks

.extern cpu_startup

.section .text

# The CPU trampoline
.align 4096
cpu_trampoline:
	jmp cpu_trampoline # TODO rm
	cli
	cld

	lgdt GDT_DESC_PHYS_PTR
	mov %cr0, %eax
	or $1, %al
	mov %eax, %cr0

	ljmp $0x8, $(TRAMPOLINE_OFFSET + (trampoline_complete_flush - cpu_trampoline))
trampoline_complete_flush:
	mov $0x10, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs
	mov %ax, %ss

	hlt # TODO rm
	# Getting the CPU core id
	mov $1, %eax
	cpuid
	shr $24, %ebx # TODO Check that the IDs are linear

	# Stack initialization
	mov $4, %eax
	mul %ebx

	mov (TRAMPOLINE_OFFSET + (trampoline_stacks - cpu_trampoline)), %eax
	add %eax, %ebx
	mov (%ebx), %esp

	# Remapping virtual memory
	# TODO Remap vmem

	add $0xc0000000, %esp
	jmp cpu_startup

# A physical address to an array containing pointers to stacks for each cores
.align 8
trampoline_stacks:
	.skip 4
