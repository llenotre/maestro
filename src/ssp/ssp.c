#include "../kernel.h"
#include "ssp.h"

__attribute__((noreturn))
void __stack_chk_fail(void)
{
	// TODO abort(); if user-space
	PANIC("Stack smashing detected!", 0);
}
