// L1 structure recovery (1 blocks)
void indirect_l1(void) {
    rax = rdi;
    edi = esi;
    esi = edx;
    __asm("jmp rax");
}
