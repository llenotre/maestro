#ifndef CPU_H
# define CPU_H

# include "../libc/stdio.h"
# include "../libc/string.h"

# define MANUFACTURER_ID_LENGTH	12

extern void cpuid_init(uint8_t *highest_call, char *manufacturer);

void cpuid();

#endif
