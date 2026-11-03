// L1 structure recovery (13 blocks)
void bytes_op_l1(void) {
    r8d = 0;
    r11 = (int64)(edx);
    r10 = rcx;
    r9d = 0;
    ecx = 0;
    if (edx > 0 /* signed */ && r11d >= 8) {
        r9 = r11;
        xmm3 = 0;
        r9 &= -0xADDR;
        xmm2 = 0;
        rdx = r10;
        do {
            xmm1 = *(u32*)(rdx);
            xmm0 = 0;
            __asm("punpcklbw xmm1, xmm0");
            ecx += 8;
            __asm("punpcklwd xmm1, xmm0");
            xmm3 += xmm1;
            xmm1 = *(u32*)((rdx + 4));
            rdx += 8;
            __asm("punpcklbw xmm1, xmm0");
            rax = rdx;
            __asm("punpcklwd xmm1, xmm0");
            rax -= r10;
            xmm2 += xmm1;
        } while (rax < r9 /* signed */);
        xmm2 += xmm3;
        xmm0 = (xmm2 >> 8);
        xmm2 += xmm0;
        xmm0 = (xmm2 >> 4);
        xmm2 += xmm0;
        r9d = xmm2;
    }
    rax = (int64)(ecx);
    edx = r8d;
    if (rax >= r11 /* signed */) {
        eax = r9d;
        return;
    }
    rcx = r11;
    rcx -= rax;
    if (rcx < 2 /* signed */) {
        ecx = (uint32)(*(u8*)((rax + r10)));
        r9d += ecx;
    } else {
        rbx = (r11 - 1);
        do {
            ecx = (uint32)(*(u8*)((r10 + rax)));
            r8d += ecx;
            ecx = (uint32)(*(u8*)(((r10 + rax) + 1)));
            edx += ecx;
            rax += 2;
        } while (rax < rbx /* signed */);
        if (rax < r11 /* signed */) {
            ecx = (uint32)(*(u8*)((rax + r10)));
            r9d += ecx;
            eax = (rdx + r8);
            eax += r9d;
            return;
        }
    }
    eax = (rdx + r8);
    eax += r9d;
    return;
}
