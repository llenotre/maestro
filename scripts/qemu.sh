#!/bin/bash

# Check that all the dependencies are installed
QEMU=qemu-system-i386
GRUB_MKRESCUE=grub-mkrescue


# Check if a program exists
# $1 => program name
check_program() {
	if ! command -v "${1}" &> /dev/null
	then
		echo "${1} could not be found."
		exit 1
	fi
}

check_program $QEMU
check_program $GRUB_MKRESCUE



# Build ISO
mkdir -p iso/boot/grub
cp $1 iso/boot/maestro
cp grub.cfg iso/boot/grub
${GRUB_MKRESCUE} -o kernel.iso iso



# Run the kernel

export QEMU_DISK=qemu_disk
export QEMUFLAGS="-device isa-debug-exit,iobase=0xf4,iosize=0x04 $QEMUFLAGS"

if [ -f $QEMU_DISK ]; then
	QEMUFLAGS="-drive file=$QEMU_DISK,format=raw $QEMUFLAGS"
fi



${QEMU} -cdrom kernel.iso $QEMUFLAGS >qemu.log 2>&1
EXIT=$?

if [ "$EXIT" -ne 33 ]; then
	exit 1
fi
