// L1 structure recovery (3 blocks)
void sub_140024cd8_l1(void) {
    rbx = rcx;
    rcx = *(u64*)(rcx);
    if (rcx != 0) {
        CloseHandle(rcx);
        *(u64*)(rbx) = 0;
    }
    return;
}
