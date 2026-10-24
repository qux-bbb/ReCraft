// L1 structure recovery (16 blocks)
void nested_loops_l1(void) {
    *(u64*)((rsp - 8)) = rbx;
    *(u64*)((rsp - 8)) = rsi;
    esi = 0;
    r11d = 0;
    ebx = ecx;
    if ((ecx <= 0 /* signed */)) {
        eax = esi;
        rsi = *(u64*)(rsp);
        rbx = *(u64*)(rsp);
        return;
    }
    xmm3 = *(__m128*)(0xADDR);
    xmm4 = 0;
    var_28 = rbp;
    xmm2 = 0;
    ebp = *(u32*)(0xADDR);
    var_30 = rdi;
    edi = 0;
    while (1) {
        edx = 0;
        xmm1 = r11d;
        __asm("pshufd xmm1, xmm1, 0xADDR");
        if (!(ebx < 8)) {
            if (!(ebp < 2 /* signed */)) {
                ecx = ebx;
                ecx = (ecx & 0xADDR);
                do {
                    xmm0 = edx;
                    eax = (rdx + 4);
                    __asm("pshufd xmm0, xmm0, 0xADDR");
                    edx = (edx + 8);
                    xmm0 = (xmm0 + xmm3);
                    xmm0 = (xmm0 * xmm1);
                    xmm4 = (xmm4 + xmm0);
                    xmm0 = eax;
                    __asm("pshufd xmm0, xmm0, 0xADDR");
                    xmm0 = (xmm0 + xmm3);
                    xmm0 = (xmm0 * xmm1);
                    xmm2 = (xmm2 + xmm0);
                } while ((edx < ecx /* signed */));
            }
        }
        r8d = 0;
        r9d = 0;
        if (!(edx >= ebx /* signed */)) {
            eax = ebx;
            eax = (eax - edx);
            if ((eax < 2 /* signed */)) {
                edx = (edx * r11d);
                esi = (esi + edx);
            } else {
                eax = edx;
                ecx = ebx;
                ecx = (ecx - edx);
                eax = (eax * r11d);
                ecx = (ecx - 2);
                ecx = (ecx >> 1);
                ecx = (ecx + 1);
                r10d = ecx;
                edx = (rdx + (rcx * 2));
                do {
                    ecx = (rax + (r11 * 1));
                    r8d = (r8d + eax);
                    r9d = (r9d + ecx);
                    eax = (eax + edi);
                    r10 = (r10 - 1);
                } while ((r10 != 1));
            }
            eax = (r9 + (r8 * 1));
            esi = (esi + eax);
        }
        r11d = (r11d + 1);
        edi = (edi + 2);
        if ((r11d >= ebx /* signed */)) break;
    }
    rdi = var_30;
    xmm2 = (xmm2 + xmm4);
    rbp = var_28;
    xmm0 = xmm2;
    xmm0 = (xmm0 >> 8);
    xmm2 = (xmm2 + xmm0);
    xmm0 = xmm2;
    xmm0 = (xmm0 >> 4);
    xmm2 = (xmm2 + xmm0);
    eax = xmm2;
    eax = (eax + esi);
    rsi = *(u64*)(rsp);
    rbx = *(u64*)(rsp);
    return;
}
