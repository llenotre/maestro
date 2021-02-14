#include <stdint.h>
#include <stddef.h>

/*
 * Copy the given memory area `src` to `dest` with size `n`.
 * If the given memory areas are overlapping, the behaviour is undefined.
 */
void *memcpy(void *dest, const void *src, size_t n)
{
	void *begin = dest;
	void *end = begin + n;

	/*while(dest < end && (((intptr_t) dest & (sizeof(long) - 1)) != 0))
		*((char *) dest++) = *((char *) src++);
	while(dest < (void *) ((intptr_t) end & ~((intptr_t) 7))
		&& (((intptr_t) dest & (sizeof(long) - 1)) == 0))
	{
		*(long *) dest = *(long *) src;
		dest += sizeof(long);
		src += sizeof(long);
	}*/
	while(dest < end)
		*((char *) dest++) = *((char *) src++);
	return begin;
}