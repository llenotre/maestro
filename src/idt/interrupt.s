.global interrupt_is_enabled

/*
 * Tells whether interrupts are enabled or not.
 */
interrupt_is_enabled:
	pushf
	pop %eax
	and $0x200, %eax
	ret
