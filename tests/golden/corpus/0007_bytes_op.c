// L1 structure recovery (13 blocks)
void bytes_op_l1(void) {
    rsp = (rsp - 8);
    r8d = (r8d ^ r8d);
    r11 = (int64*)(edx);
    r10 = rcx;
    r9d = r8d;
    ecx = r8d;
    __asm("if (jle short 0xADDR /* unmodelled flags */) goto 0xADDR");
    if (!(r11d < 8)) {
        r9 = r11;
        __asm("xorps xmm3, xmm3");
        r9 = (r9 & 0xADDR);
        __asm("xorps xmm2, xmm2");
        rdx = r10;
        while (1) {
            __asm("movd xmm1, [rdx]");
            __asm("xorps xmm0, xmm0");
            __asm("punpcklbw xmm1, xmm0");
            ecx = (ecx + 8);
            __asm("punpcklwd xmm1, xmm0");
            __asm("paddd xmm3, xmm1");
            __asm("movd xmm1, [rdx+0xADDR]");
            rdx = (rdx + 8);
            __asm("punpcklbw xmm1, xmm0");
            rax = rdx;
            __asm("punpcklwd xmm1, xmm0");
            rax = (rax - r10);
            __asm("paddd xmm2, xmm1");
            if ((rax >= r9 /* signed */)) break;
        }
        __asm("paddd xmm2, xmm3");
        __asm("movdqa xmm0, xmm2");
        __asm("psrldq xmm0, 0xADDR");
        __asm("paddd xmm2, xmm0");
        __asm("movdqa xmm0, xmm2");
        __asm("psrldq xmm0, 0xADDR");
        __asm("paddd xmm2, xmm0");
        __asm("movd r9d, xmm2");
    }
    rax = (int64*)(ecx);
    edx = r8d;
    if ((rax >= r11 /* signed */)) {
        eax = r9d;
        rsp = (rsp + 8);
        return;
    }
    rcx = r11;
    rcx = (rcx - rax);
    if ((rcx < 2 /* signed */)) {
        ecx = (uint32*)(*(u8*)((rax + (r10 * 1))));
        r9d = (r9d + ecx);
    } else {
        *(u64*)(rsp) = rbx;
        rbx = (r11 - 1);
        while (1) {
            ecx = (uint32*)(*(u8*)((r10 + (rax * 1))));
            r8d = (r8d + ecx);
            ecx = (uint32*)(*(u8*)(((r10 + (rax * 1)) + 1)));
            edx = (edx + ecx);
            rax = (rax + 2);
            if ((rax >= rbx /* signed */)) break;
        }
        rbx = *(u64*)(rsp);
        if ((rax >= r11 /* signed */)) goto loc_ADDR;
    }
    eax = (rdx + (r8 * 1));
    eax = (eax + r9d);
    rsp = (rsp + 8);
    return;
}
