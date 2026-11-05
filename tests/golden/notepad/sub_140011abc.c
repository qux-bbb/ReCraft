// L1 structure recovery (4 blocks)
void sub_140011abc_l1(void) {
    r9 = rcx;
    if (r8d != 0) {
        r9 -= rdx;
        r10d = r8d;
        do {
            ecx = (uint32)(*(u16*)(rdx));
            r8d = (uint32)(*(u8*)((rdx + 1)));
            ecx *= 0xADDR;
            r8w += cx;
            *(u16*)((r9 + rdx)) = r8w;
            rdx += 2;
            r10 -= 1;
        } while (r10 != 0);
    }
    return;
}
