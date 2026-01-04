#!/bin/bash

cargo build --release || exit 1

# QEMU file path
QEMU=qemu-system-riscv32

# Start QEMU
$QEMU -machine virt -bios default -nographic -serial mon:stdio --no-reboot \
    -kernel target/riscv32i-unknown-none-elf/release/orca1000
