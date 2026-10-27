.set noreorder
.option pic0
.text
.globl main
.ent main
main:
 addu $s0,$a0,$zero
 lui $a0,%hi(path)
 addiu $a0,$a0,%lo(path)
 addiu $a1,$zero,577
 addiu $v0,$zero,2
 syscall
 addu $s1,$v0,$zero
 addu $a0,$s1,$zero
 lui $a1,%hi(header)
 addiu $a1,$a1,%lo(header)
 addiu $a2,$zero,54
 addiu $v0,$zero,1
 syscall
 addu $a0,$s1,$zero
 addu $a1,$s0,$zero
 addiu $a2,$zero,16
 addiu $v0,$zero,1
 syscall
 addu $a0,$s1,$zero
 jr $ra
 nop
.end main
.data
path: .asciz "frame.bmp"
.align 2
header: .byte 66,77,70,0,0,0,0,0,0,0,54,0,0,0,40,0,0,0,2,0,0,0,254,255,255,255,1,0,32,0,0,0,0,0,16,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
