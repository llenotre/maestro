/*
 * This file implements CPU-related functions.
 */

.global cpu_wait
.global cpu_loop
.global cpu_loop_reset
.global cpu_halt

.section .text

/*
 * Makes the current CPU wait for an interrupt, then returns.
 * This function enables interrupts.
 */
cpu_wait:
	sti
	hlt
	ret

/*
 * Makes the current CPU loop and process every interrupts indefinitely.
 */
cpu_loop:
	sti
	hlt
	jmp cpu_loop

/*
 * Resets the stack to the given value, then calls `cpu_loop`.
 */
cpu_loop_reset:
	mov 4(%esp), %esp
	jmp cpu_loop

/*
 * Halts the current CPU until reboot.
 */
cpu_halt:
	cli
	hlt
	jmp cpu_halt
