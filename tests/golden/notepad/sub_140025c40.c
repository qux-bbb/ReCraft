// L1 structure recovery (1 blocks)
void sub_140025c40_l1(void) {
    *(u32*)((rcx + 0xADDR)) = r8d;
    rcx = *(u64*)((rcx + 0xADDR));
    SetEvent(*(u64*)((rcx + 0xADDR)));
    eax = 0;
    return;
}
