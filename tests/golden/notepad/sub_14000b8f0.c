// L1 structure recovery (4 blocks)
void sub_14000b8f0_l1(void) {
    rcx = *(u64*)((rcx + 0xADDR));
    if (rcx == 0) {
        eax = 0xADDR;
    } else {
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010();
    }
    return;
}
