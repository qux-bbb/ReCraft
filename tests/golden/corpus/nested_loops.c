// L1 structure recovery (16 blocks)
void nested_loops_l1(void) {
    esi = 0;
    r11d = 0;
    ebx = ecx;
    if (ecx <= 0 /* signed */) {
        eax = esi;
        rsi = *(u64*)(rsp);
        rbx = *(u64*)(rsp);
        return;
    }
    xmm3 = *(__m128*)(0xADDR);
    xmm4 = 0;
    xmm2 = 0;
    ebp = *(u32*)(0xADDR);
    edi = 0;
    while (1) {
        edx = 0;
        xmm1 = r11d;
        __asm("pshufd xmm1, xmm1, 0xADDR");
        if (ebx >= 8) {
            if (ebp >= 2 /* signed */) {
                ecx = ebx;
                ecx &= 0xADDR;
                do {
                    xmm0 = edx;
                    eax = (rdx + 4);
                    __asm("pshufd xmm0, xmm0, 0xADDR");
                    edx += 8;
                    xmm0 += xmm3;
                    xmm0 *= xmm1;
                    xmm4 += xmm0;
                    xmm0 = eax;
                    __asm("pshufd xmm0, xmm0, 0xADDR");
                    xmm0 += xmm3;
                    xmm0 *= xmm1;
                    xmm2 += xmm0;
                } while ((edx < ecx /* signed */));
            }
        }
        r8d = 0;
        r9d = 0;
        if (edx < ebx /* signed */) {
            eax = ebx;
            eax -= edx;
            if (eax < 2 /* signed */) {
                edx *= r11d;
                esi += edx;
            } else {
                eax = edx;
                ecx = ebx;
                ecx -= edx;
                eax *= r11d;
                ecx -= 2;
                ecx >>= 1;
                ecx += 1;
                r10d = ecx;
                edx = (rdx + (rcx * 2));
                do {
                    ecx = (rax + (r11 * 1));
                    r8d += eax;
                    r9d += ecx;
                    eax += edi;
                    r10 -= 1;
                } while ((r10 != 1));
            }
            eax = (r9 + (r8 * 1));
            esi += eax;
        }
        r11d += 1;
        edi += 2;
        if ((r11d >= ebx /* signed */)) break;
    }
    xmm2 += xmm4;
    xmm0 = xmm2;
    xmm0 >>= 8;
    xmm2 += xmm0;
    xmm0 = xmm2;
    xmm0 >>= 4;
    xmm2 += xmm0;
    eax = xmm2;
    eax += esi;
    rsi = *(u64*)(rsp);
    rbx = *(u64*)(rsp);
    return;
}
