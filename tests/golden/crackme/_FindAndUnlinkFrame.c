// L1 structure recovery (7 blocks)
void _FindAndUnlinkFrame_l1(void) {
    rdi = rcx;
    __vcrt_getptd(rcx);
    if (rdi != *(u64*)((rax + 0xADDR))) {
        abort();
    }
    __vcrt_getptd(rcx);
    rdx = *(u64*)((rax + 0xADDR));
    if (rdx == 0) {
        abort();
        return;
    }
    while (1) {
        rbx = *(u64*)((rdx + 8));
        if (rdi == rdx) break;
        rdx = rbx;
        if (rbx == 0) {
            abort();
            return;
        }
        continue;
    }
    __vcrt_getptd(rcx);
    *(u64*)((rax + 0xADDR)) = rbx;
    return;
}
