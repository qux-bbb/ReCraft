// L1 structure recovery (17 blocks)
void nested_loops_l1(void) {
    var_c = ebp;
    var_8 = esi;
    esi = var_20;
    edx = 0;
    ebp = 0;
    var_10 = 0;
    if (esi <= 0 /* signed */) {
        esi = var_8;
        eax = edx;
        ebp = var_c;
        return;
    }
    xmm3 = *(__m128*)(0xADDR);
    xmm4 = 0;
    xmm2 = 0;
    do {
        xmm0 = ebp;
        eax = 0;
        __asm("pshufd xmm1, xmm0, 0xADDR");
        if (esi >= 8 && *(u32*)(0xADDR) >= 2 /* signed */) {
            edx = (esi & -0xADDR);
            do {
                xmm0 = eax;
                ecx = (eax + 4);
                __asm("pshufd xmm0, xmm0, 0xADDR");
                eax += 8;
                xmm0 += xmm3;
                xmm0 *= xmm1;
                xmm4 += xmm0;
                xmm0 = ecx;
                __asm("pshufd xmm0, xmm0, 0xADDR");
                xmm0 += xmm3;
                xmm0 *= xmm1;
                xmm2 += xmm0;
            } while (eax < edx /* signed */);
            edx = var_10;
        }
        edi = 0;
        ebx = 0;
        if (eax < esi /* signed */) {
            ecx = esi;
            ecx -= eax;
            if (ecx < 2 /* signed */) {
                eax *= ebp;
                edx += eax;
            } else {
                esi -= eax;
                edx = eax;
                esi -= 2;
                edx *= ebp;
                esi >>= 1;
                esi += 1;
                eax += (esi * 2);
                var_14 = eax;
                eax = (ebp * 2);
                do {
                    ecx = (edx + ebp);
                    edi += edx;
                    ebx += ecx;
                    edx += eax;
                    esi -= 1;
                } while (esi != 0);
                eax = var_14;
                esi = var_20;
                edx = var_10;
            }
            eax = (ebx + edi);
            edx += eax;
            var_10 = edx;
        }
        ebp += 1;
    } while (ebp < esi /* signed */);
    xmm2 += xmm4;
    xmm0 = (xmm2 >> 8);
    xmm2 += xmm0;
    xmm0 = (xmm2 >> 4);
    xmm2 += xmm0;
    eax = xmm2;
    esi = var_8;
    ebp = var_c;
    eax += edx;
    return;
}
