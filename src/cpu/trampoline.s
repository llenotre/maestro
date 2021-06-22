/*
 * This file implements the trampoline code used to boot CPU cores for multicore processing.
 * The below code is real mode assembly, thus before jumping to the real kernel code, it is
 * required to switch to protected mode.
 *
 * The code will be relocated at an address at which it can be accessed in real mode.
 */

.global cpu_trampoline

.extern cpu_startup

.section .text

cpu_trampoline:
	cli
	cld

	lgdt GDT_DESC_PHYS_PTR
	mov %cr0, %eax
	or $1, %al
	mov %eax, %cr0

	jmp $0x8, $complete_flush
complete_flush:
	mov $0x10, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs
	mov %ax, %ss

.align 32
init_stack:
	mov $1, %eax
	cpuid
	shr $24, %ebx

	# TODO Setup stack

	jmp cpu_startup
