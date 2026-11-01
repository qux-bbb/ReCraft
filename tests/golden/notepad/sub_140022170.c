// L1 structure recovery (3 blocks)
void sub_140022170_l1(void) {
    ebx = edx;
    rdi = rcx;
    sub_1400220b0(rcx);
    if ((bl & 1) != 0) {
        sub_140001c84(rdi, 0xADDR);
    }
    rax = rdi;
    return;
}
