// L1 structure recovery (5 blocks)
void sub_140023b38_l1(void) {
    rbx = rcx;
    rcx = *(u64*)(rcx);
    if (rcx != 0) {
        CloseHandle(rcx);
        *(u64*)(rbx) = 0;
    }
    rcx = *(u64*)((rbx + 8));
    if (rcx != 0) {
        CloseHandle(*(u64*)((rbx + 8)));
        *(u64*)((rbx + 8)) = 0;
    }
    return;
}
