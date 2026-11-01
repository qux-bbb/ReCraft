// L1 structure recovery (3 blocks)
void sub_140025520_l1(void) {
    *(u32*)((rcx + 0xADDR)) = 0xADDR;
    rbx = rcx;
    if ((dl & 1) != 0) {
        sub_1400025e0();
    }
    rax = rbx;
    return;
}
