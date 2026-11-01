// L1 structure recovery (5 blocks)
void internal/abi.(*IntArgRegBitmap).Set_l1(void) {
    rcx = rbx;
    rbx >>= 0xADDR;
    rbx >>= 0xADDR;
    rdx = ((rcx + rbx) >> 3);
    if (rdx >= 2) {
        runtime.panicBounds(rcx, rdx, r8, r9);
    }
    rbx = rdx;
    rdx <<= 3;
    rcx -= rdx;
    edx = (uint32)(*(u8*)((rax + rbx)));
    if (rcx < 0 /* signed */) {
        runtime.panicshift();
    }
    edx |= (1 << (ecx & 0xADDR));
    *(u8*)((rax + rbx)) = dl;
    return;
}
