// L1 structure recovery (1 blocks)
void sub_1400052b0_l1(void) {
    r8 = rcx;
    *(u32*)(rcx) = edx;
    sub_140005cc0(edx);
    *(u32*)((r8 + 8)) = 0;
    *(u32*)((r8 + 4)) = eax;
    rax = r8;
    return;
}
