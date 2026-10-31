// L1 structure recovery (4 blocks)
void sub_140002aa0_l1(void) {
    rbx = rcx;
    *(u64*)(rcx) = rdx;
    sub_14000217c(rcx);
    if (rbx >= *(u64*)((rax + 0xADDR))) {
        ecx = 0;
    } else {
        sub_14000217c(rcx);
        rcx = *(u64*)((rax + 0xADDR));
    }
    *(u64*)((rbx + 8)) = rcx;
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = rbx;
    rax = rbx;
    return;
}
