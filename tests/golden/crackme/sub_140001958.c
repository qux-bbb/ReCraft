// L1 structure recovery (7 blocks)
void sub_140001958_l1(void) {
    rbx = *(u64*)(rcx);
    rdi = rcx;
    if ((*(u32*)(rbx) != 0xADDR) || (*(u32*)((rbx + 0xADDR)) != 4)) {
        return;
    }
    edx = *(u32*)((rbx + 0xADDR));
    if (edx == 0xADDR) {
        sub_140002088(rcx);
        *(u64*)(0) = rbx;
        rbx = *(u64*)((rdi + 8));
        sub_14000209c(rcx);
        *(u64*)(0) = rbx;
        sub_1400087a0();
    }
    eax = (rdx - 0xADDR);
    if ((eax <= 1) || (edx == 0xADDR)) {
        sub_140002088(rcx);
        *(u64*)(rax) = rbx;
        rbx = *(u64*)((rdi + 8));
        sub_14000209c(rcx);
        *(u64*)(rax) = rbx;
        sub_1400087a0();
        return;
    }
}
