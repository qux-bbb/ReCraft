// L1 structure recovery (1 blocks)
void indirect_l1(void) {
    eax = edx;
    r9 = rcx;
    ecx = eax;
    edx = r8d;
    __asm("jmp r9");
}
