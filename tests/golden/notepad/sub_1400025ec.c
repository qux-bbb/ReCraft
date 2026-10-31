// L1 structure recovery (8 blocks)
void sub_1400025ec_l1(void) {
    rbx = rcx;
    while (1) {
        sub_1400029a0();
        if (rax != 0) break;
        rcx = rbx;
        sub_1400028c8(rbx);
        rcx = rbx;
    }
    return;
}
