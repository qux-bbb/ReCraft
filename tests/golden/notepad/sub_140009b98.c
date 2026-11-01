// L1 structure recovery (3 blocks)
void sub_140009b98_l1(void) {
    rbx = rcx;
    rcx = *(u64*)(rcx);
    if (rcx != 0) {
        *(u64*)(rbx) = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010();
    }
    rax = rbx;
    return;
}
