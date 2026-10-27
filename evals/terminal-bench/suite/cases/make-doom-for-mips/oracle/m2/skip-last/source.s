.set noreorder
.option pic0
.text
.globl main
.ent main
main:
 addiu $a1,$a1,-1
 addu $v0,$zero,$zero
loop:
 beq $a1,$zero,done
 nop
 lbu $t0,0($a0)
 addu $v0,$v0,$t0
 addiu $a0,$a0,1
 addiu $a1,$a1,-1
 b loop
 nop
done:
 jr $ra
 nop
.end main
