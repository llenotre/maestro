/*
 * This file implements functions related to the CPUID instruction (x86).
 */

.global get_current_apic

.section .text

/*
 * Returns the current CPU id.
 */
get_current_apic:
	mov $1, %eax
	cpuid
	shr $24, %ebx
	mov %ebx, %eax
	ret
