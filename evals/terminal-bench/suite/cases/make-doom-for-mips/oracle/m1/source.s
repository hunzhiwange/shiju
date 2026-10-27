# Verifier-owned synthetic probe, frozen independently.
.set noreorder
.option pic0
.text
.globl main
.ent main
main:
 addiu $v0,$zero,42
 jr $ra
 nop
.end main
