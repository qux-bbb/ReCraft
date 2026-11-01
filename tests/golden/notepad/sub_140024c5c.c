// L1 structure recovery (3 blocks)
void sub_140024c5c_l1(void) {
    rbx = rcx;
    rcx = *(u64*)(rcx);
    if (rcx != 0) {
        LocalFree();
        *(u64*)(rbx) = 0;
    }
    return;
}
