// L1 structure recovery (5 blocks)
void sub_1400255d0_l1(void) {
    rbx = rcx;
    *(u64*)(rcx) = off_1400294d8;
    edi = edx;
    *(u64*)((rcx + 8)) = off_1400293f8;
    rcx = *(u64*)((rcx + 0xADDR));
    CloseHandle(*(u64*)((rcx + 0xADDR)));
    *(u32*)((rbx + 0xADDR)) = 0xADDR;
    rcx = *(u64*)((rbx + 0xADDR));
    if (rcx != 0) {
        *(u64*)((rbx + 0xADDR)) = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010();
    }
    if ((dil & 1) != 0) {
        rcx = rbx;
        sub_1400025e0(rbx);
    }
    rax = rbx;
    return;
}
