/* SPDX-License-Identifier: LGPL-3.0-or-later */
.bss
.align 8
saved_frame: .quad 0
.text
.global enter_enclave
.type enter_enclave,@function
enter_enclave:
 push %rbp
 push %rbx
 push %r12
 push %r13
 push %r14
 push %r15
 sub $40,%rsp
 mov %rsp,saved_frame(%rip)
 mov %rdi,0(%rsp)
 mov %rsi,8(%rsp)
 mov %rdx,16(%rsp)
.Lenter:
 mov 0(%rsp),%rbx
 mov 8(%rsp),%rdi
 mov 16(%rsp),%rsi
 lea .Laep(%rip),%rcx
 mov $2,%eax
 .byte 0x0f,0x01,0xd7
.Lreturned:
 cmp $-1,%rdi
 je .Ldone
 call handle_ocall
 mov saved_frame(%rip),%rsp
 movq $-2,8(%rsp)
 mov %rax,16(%rsp)
 jmp .Lenter
.Ldone:
 mov saved_frame(%rip),%rsp
 mov %rsi,%rax
 add $40,%rsp
 pop %r15
 pop %r14
 pop %r13
 pop %r12
 pop %rbx
 pop %rbp
 ret
.Laep:
 .byte 0x0f,0x01,0xd7
 jmp .Lreturned
.size enter_enclave,.-enter_enclave
.section .note.GNU-stack,"",@progbits
