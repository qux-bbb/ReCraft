// L1 structure recovery (11 blocks)
void internal/abi.BoundsDecode_l1(void) {
    while (1) {
        if (rsp <= *(u64*)((r14 + 0xADDR))) {
            runtime.morestack_noctxt();
            continue;
        }
        rdx = rax;
        rax >>= 0xADDR;
        rcx = rax;
        rax = 0xADDR;
        rbx = rdx;
        __asm("imul rdx");
        rdi = (rbx + rdx);
        rdi >>= 3;
        rdi -= rcx;
        rdx = (rdi + (rdi * 8));
        rbx -= rdx;
        rcx = rdi;
        rdi >>= 2;
        if ((ecx & 1) == 0) {
            edi &= 0xADDR;
            r9 = rcx;
            r9 >>= 7;
            rdx = rdi;
            edi = 0;
        } else {
            rdx = rcx;
            rdx >>= 3;
            edi &= 1;
            edx &= 0xADDR;
            r9 = rcx;
            r9 >>= 7;
        }
        r10 = rcx;
        r10 >>= 1;
        if ((r10d & 1) == 0) {
            r11 = r9;
            r9d &= 0xADDR;
            r11 >>= 5;
        } else {
            r11 = r9;
            r9d &= 0xADDR;
            r11 >>= 4;
        }
        if (r11 == 0) break;
        rax = off_1400bd6e0;
        rbx = "H";
        runtime.gopanic(rcx);
    }
    ecx &= 1;
    r10d &= 1;
    rax = rbx;
    ebx = edi;
    edi = r10d;
    rsi = rdx;
    r8 = r9;
    return;
}
