// L1 structure recovery (3 blocks)
void sub_140004230_l1(void) {
    rax = off_140029090;
    rbx = rcx;
    *(u64*)(rcx) = rax;
    if ((dl & 1) != 0) {
        sub_140001c84(rcx, 0xADDR);
    }
    rax = rbx;
    return;
}
