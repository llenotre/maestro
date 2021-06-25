/*
 * This file implements the trampoline code used to boot CPU cores for multicore processing.
 * The below code is real mode assembly, thus before jumping to the real kernel code, it is
 * required to switch to protected mode.
 *
 * The code will be relocated at an address at which it can be accessed in real mode.
 */

.set TRAMPOLINE_OFFSET, 0x8000

.global cpu_trampoline

.extern cpu_startup

.section .text

.set cpu_trampoline, TRAMPOLINE_OFFSET
# The CPU trampoline
cpu_trampoline:
	cli
	cld

	lgdt GDT_DESC_PHYS_PTR
	mov %cr0, %eax
	or $1, %al
	mov %eax, %cr0

	jmp $0x8, $(TRAMPOLINE_OFFSET + (trampoline_complete_flush - cpu_trampoline))
trampoline_complete_flush:
	mov $0x10, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs
	mov %ax, %ss

	# Getting the CPU core id
	mov $1, %eax
	cpuid
	shr $24, %ebx

	# Stack initialization
	# TODO Setup stack

	# Remapping virtual memory
	# TODO Remap vmem

	jmp cpu_startup
