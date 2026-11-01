// L1 structure recovery (5 blocks)
void sub_14000a150_l1(void) {
    rbx = rcx;
    *(u32*)((rcx + 0xADDR)) = 0xADDR;
    rcx = *(u64*)((rcx + 0xADDR));
    edi = edx;
    if (rcx != 0) {
        *(u64*)((rbx + 0xADDR)) = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010();
    }
    if ((dil & 1) != 0) {
        sub_140001c84(rbx, 0xADDR);
    }
    rax = rbx;
    return;
}
