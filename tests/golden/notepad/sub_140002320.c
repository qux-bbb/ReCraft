// L1 structure recovery (6 blocks)
void sub_140002320_l1(void) {
    rbx = *(u64*)(rcx);
    rdi = rcx;
    if ((*(u32*)(rbx) != 0xADDR) || (*(u32*)((rbx + 0xADDR)) != 4)) {
        eax = 0;
        return;
    }
    edx = *(u32*)((rbx + 0xADDR));
    eax = (rdx - 0xADDR);
    if (eax <= 2) {
        sub_1400029c4();
        *(u64*)(rax) = rbx;
        rbx = *(u64*)((rdi + 8));
        sub_1400029d0();
        *(u64*)(rax) = rbx;
        sub_1400029ac();
    }
    if (edx == 0xADDR) {
        sub_1400029c4();
        *(u64*)(rax) = rbx;
        rbx = *(u64*)((rdi + 8));
        sub_1400029d0();
        *(u64*)(rax) = rbx;
        sub_1400029ac();
        return;
    }
}
