// L1 structure recovery (3 blocks)
void sub_140002c10_l1(void) {
    r9 = (int64)(*(u32*)((r8 + 0xADDR)));
    r10 = r8;
    rax = *(u64*)(rcx);
    if (*(u32*)((r9 + rax)) == -0xADDR) {
        sub_140002cb4(r10, rdx, *(u64*)(rdx));
    }
    return;
}
