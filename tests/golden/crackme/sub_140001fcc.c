// L1 structure recovery (7 blocks)
void sub_140001fcc_l1(void) {
    rbx = rcx;
    sub_14000217c(rcx);
    rdx = *(u64*)((rax + 0xADDR));
    while (rdx != 0) {
        if (*(u64*)(rdx) != rbx) {
            rdx = *(u64*)((rdx + 8));
        }
    }
    eax = (rdx + 1);
    return;
}
