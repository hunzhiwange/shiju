# Synthetic MIPS diagnostic ABI
ELF32 little-endian EM_MIPS executable; PT_LOAD segments, e_entry is entry.
Registers and memory are unsigned 32-bit, register zero is immutable.
Standard MIPS32 integer instructions and one branch delay slot apply.
A function returns using jr $ra; initial $ra = 0xfffffffc.
There is no Linux runtime or dynamic loader. Use .set noreorder, .option pic0,
.text, .globl main, .ent main; link with -e main and -Ttext 0x10000.
Custom VM syscalls: $v0=2 opens the NUL-terminated path at $a0, flags $a1=577
(write/create/truncate), returns fd=3. $v0=1 writes $a2 bytes from $a1 to
fd $a0 (1=stdout,3=file); return byte count. $v0=3 closes fd $a0 and
flushes the file. Unsupported calls fail. Paths must be relative frame.bmp.
GNU mipsel-linux-gnu-as and mipsel-linux-gnu-ld are available offline.
