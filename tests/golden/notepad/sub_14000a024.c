// L1 structure recovery (3 blocks)
void sub_14000a024_l1(void) {
    rcx = *(u64*)(rcx);
    if (rcx != 0) {
        CoTaskMemFree(rcx);
    }
    return;
}
