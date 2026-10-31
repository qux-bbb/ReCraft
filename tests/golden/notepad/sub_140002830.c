// L1 structure recovery (3 blocks)
void sub_140002830_l1(void) {
    rax = "0(";
    rbx = rcx;
    *(u64*)(rcx) = rax;
    if ((dl & 1) != 0) {
        sub_1400025e0();
    }
    rax = rbx;
    return;
}
