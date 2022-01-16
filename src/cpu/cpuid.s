/*
 * This file implements functions related to the CPUID instruction (x86).
 */

.global cpuid_clock_ratios
.global cpuid_has_sse
.global get_current_apic
.global msr_exist
.global msr_read
.global msr_write

.section .text

/*
 * Tells whether MSR exist on the current core.
 */
msr_exist:
	push %ebx

	mov $1, %eax
	cpuid
	shr $5, %ebx
	and $1, %ebx

	mov %ebx, %eax
	pop %ebx
	ret

/*
 * Reads the value of the given MSR.
 */
msr_read:
	push %ebp
	mov %esp, %ebp

	mov 8(%ebp), %ecx # msr
	rdmsr
	mov 12(%ebp), %ecx # lo
	mov %eax, (%ecx)
	mov 16(%ebp), %ecx # hi
	mov %edx, (%ecx)

	mov %ebp, %esp
	pop %ebp
	ret

/*
 * Writes the given value to the given MSR.
 */
msr_write:
	push %ebp
	mov %esp, %ebp

	mov 8(%ebp), %ecx # msr
	mov 12(%ebp), %eax # lo
	mov 16(%ebp), %edx # hi
	wrmsr

	mov %ebp, %esp
	pop %ebp
	ret

/*
 * Returns the current CPU id.
 */
get_current_apic:
	push %ebx

	mov $0x1, %eax
	cpuid
	shr $24, %ebx

	mov %ebx, %eax

	pop %ebx
	ret

/*
 * Tells whether the CPU has SSE.
 */
cpuid_has_sse:
	push %ebx

	mov $0x1, %eax
	cpuid
	shr $25, %edx
	and $0x1, %edx
	mov %edx, %eax

	pop %ebx
	ret

/*
 * Returns the CPU's clock ratios.
 */
cpuid_clock_ratios:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x15, %eax
	cpuid
	mov %ecx, 8(%ebp)
	mov %ebx, 12(%ebp)
	mov %eax, 16(%ebp)

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret
