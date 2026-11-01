// L1 structure recovery (3 blocks)
void internal/abi.(*IntArgRegBitmap).Get_l1(void) {
    rcx = rbx;
    rbx >>= 0xADDR;
    rbx >>= 0xADDR;
    rdx = ((rcx + rbx) >> 3);
    rbx = rdx;
    rdx <<= 3;
    rcx -= rdx;
    edx = (uint32)(*(u8*)((rax + rbx)));
    if (rcx < 0 /* signed */) {
        runtime.panicshift();
    }
    ebx = 1;
    ebx <<= cl;
    al = (uint8)(((bl & dl) != 0));
    return;
}
