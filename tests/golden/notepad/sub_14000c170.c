// L1 structure recovery (3 blocks)
void sub_14000c170_l1(void) {
    rbx = rcx;
    *(u64*)(rcx) = off_140029090;
    if ((dl & 1) != 0) {
        sub_140001c84(rcx, 0xADDR);
    }
    rax = rbx;
    return;
}
