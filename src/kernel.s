.global kernel_begin
.global kernel_end

/*
 * The kernel begin symbol, giving the pointer to the begin of the kernel image
 * in the virtual memory. This memory location should never be accessed using
 * this symbol.
 */
kernel_begin:

/*
 * The kernel end symbol, giving the pointer to the end of the kernel image in
 * the virtual memory. This memory location should never be accessed using this
 * symbol.
 */
kernel_end:
