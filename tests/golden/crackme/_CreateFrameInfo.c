// L1 structure recovery (4 blocks)
void _CreateFrameInfo_l1(void) {
    rbx = rcx;
    *(u64*)(rcx) = rdx;
    __vcrt_getptd(rcx);
    if (rbx >= *(u64*)((rax + 0xADDR))) {
        ecx = 0;
    } else {
        __vcrt_getptd(rcx);
        rcx = *(u64*)((rax + 0xADDR));
    }
    *(u64*)((rbx + 8)) = rcx;
    __vcrt_getptd(rcx);
    *(u64*)((rax + 0xADDR)) = rbx;
    rax = rbx;
    return;
}
