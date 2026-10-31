// L1 structure recovery (3 blocks)
void sub_140004390_l1(void) {
    rbx = rcx;
    if (*(u8*)((rcx + 8)) != 0) {
        rcx = *(u64*)(rcx);
        sub_1400087c0(rcx);
    }
    *(u8*)((rbx + 8)) = 0;
    *(u64*)(rbx) = 0;
    return;
}
