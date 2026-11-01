// L1 structure recovery (5 blocks)
void sub_1400220b0_l1(void) {
    rax = off_140029258;
    rbx = rcx;
    *(u64*)(rcx) = rax;
    rcx = *(u64*)((rcx + 0xADDR));
    if (rcx != 0) {
        CoTaskMemFree(rcx);
    }
    rcx = *(u64*)((rbx + 8));
    if (rcx != 0) {
        CoTaskMemFree(*(u64*)((rbx + 8)));
    }
    return;
}
