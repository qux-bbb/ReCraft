// L1 structure recovery (3 blocks)
void sub_14000aaa0_l1(void) {
    rcx = *(u64*)((rcx + 0xADDR));
    if (rcx == 0) {
        eax = 0xADDR;
        return;
    }
    rax = *(u64*)(rcx);
    rax = *(u64*)((rax + 0xADDR));
    return sub_140028010();
}
