// L1 structure recovery (13 blocks)
void bytes_op_l1(void) {
    rsp = (rsp - 8);
    r8d = (r8d ^ r8d);
    r11 = (int64*)(edx);
    r10 = rcx;
    r9d = r8d;
    ecx = r8d;
    if (!((edx & edx) <= 0 /* signed */)) {
        if (!(r11d < 8)) {
            r9 = r11;
            xmm3 = (xmm3 ^ xmm3);
            r9 = (r9 & 0xADDR);
            xmm2 = (xmm2 ^ xmm2);
            rdx = r10;
            do {
                xmm1 = *(u32*)(rdx);
                xmm0 = (xmm0 ^ xmm0);
                __asm("punpcklbw xmm1, xmm0");
                ecx = (ecx + 8);
                __asm("punpcklwd xmm1, xmm0");
                xmm3 = (xmm3 + xmm1);
                xmm1 = *(u32*)((rdx + 4));
                rdx = (rdx + 8);
                __asm("punpcklbw xmm1, xmm0");
                rax = rdx;
                __asm("punpcklwd xmm1, xmm0");
                rax = (rax - r10);
                xmm2 = (xmm2 + xmm1);
            } while ((rax < r9 /* signed */));
            xmm2 = (xmm2 + xmm3);
            xmm0 = xmm2;
            xmm0 = (xmm0 >> 8);
            xmm2 = (xmm2 + xmm0);
            xmm0 = xmm2;
            xmm0 = (xmm0 >> 4);
            xmm2 = (xmm2 + xmm0);
            r9d = xmm2;
        }
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
        do {
            ecx = (uint32*)(*(u8*)((r10 + (rax * 1))));
            r8d = (r8d + ecx);
            ecx = (uint32*)(*(u8*)(((r10 + (rax * 1)) + 1)));
            edx = (edx + ecx);
            rax = (rax + 2);
        } while ((rax < rbx /* signed */));
        rbx = *(u64*)(rsp);
    }
    eax = (rdx + (r8 * 1));
    eax = (eax + r9d);
    rsp = (rsp + 8);
    return;
}
