// L1 structure recovery (9 blocks)
void internal/abi.writeVarint_l1(void) {
    ecx = 0;
    while (1) {
        rdx = rdi;
        rdi >>= 7;
        edx &= 0xADDR;
        if (rdi == 0) break;
        if (rbx > rcx) {
            edx |= -0xADDR;
            *(u8*)((rax + rcx)) = dl;
            rcx += 1;
        }
    }
    if (rbx <= rcx) {
        runtime.panicBounds(rcx, rdx, r8, r9);
        runtime.panicBounds(rcx, rdx, r8, r9);
    }
    *(u8*)((rax + rcx)) = dl;
    rax = (rcx + 1);
    return;
}
