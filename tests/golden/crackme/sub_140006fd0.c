// L1 structure recovery (70 blocks)
void sub_140006fd0_l1(void) {
    r10 = rdx;
    rsi = 0xADDR;
    r10d &= 0xADDR;
    rbx = rdx;
    rbx -= r10;
    edi = 0;
    r11 = rdx;
    r8 = rcx;
    xmm3 = 0;
    rax = (r10 - 1);
    xmm1 = *(__m128*)(rbx);
    if (rax > 0xADDR) {
        loc_ADDR:
        xmm0 = 0;
        r9d = 0xADDR;
        __asm("pcmpeqb xmm0, xmm1");
        __asm("pmovmskb eax, xmm0");
        if (eax == 0) {
            if (r10 != 0) {
                xmm2 = *(__m128*)((rbx + 0xADDR));
                xmm0 = xmm2;
                __asm("pcmpeqb xmm0, xmm3");
                __asm("pmovmskb eax, xmm0");
                if (eax != 0) {
                    ecx = ctz(eax);
                    eax = ecx;
                    rax -= r10;
                    rax += 0xADDR;
                    if (rax > 0xADDR) {
                        rdx = r11;
                        rcx = r8;
                        return sub_140006f30();
                    }
                    r9d -= ecx;
                    if (r9d > 0xADDR) {
                        loc_ADDR:
                        __asm("orps xmm1, xmm2");
                        loc_ADDR:
                        eax = (uint32)(*(u8*)(r8));
                        if (al == 0) {
                            eax = 0;
                            return;
                        }
                        while (1) {
                            eax = (int32)(al);
                            xmm0 = eax;
                            __asm("punpcklbw xmm0, xmm0");
                            __asm("punpcklbw xmm0, xmm0");
                            __asm("pshufd xmm0, xmm0, 0xADDR");
                            __asm("pcmpeqb xmm0, xmm1");
                            __asm("pmovmskb eax, xmm0");
                            if (eax != 0) break;
                            eax = (uint32)(*(u8*)((r8 + 1)));
                            r8 += 1;
                            if (al != 0) continue;
                        }
                        rax = r8;
                        return;
                    }
                    ecx = *(u32*)(((0xADDR + (r9 * 4)) + 0xADDR));
                    rcx += 0xADDR;
                    __asm("jmp rcx");
                }
            }
            if (*(u8*)((rbx + 1)) == dil) goto loc_ADDR;
        }
        edx = ctz(eax);
        if (r10 != 0) {
            ecx = 0xADDR;
            eax = edx;
            rcx -= r10;
            dil = (uint8)((rax < rcx));
        } else {
            edi = (r9 - 0xADDR);
        }
        eax = r9d;
        eax -= edx;
        if (eax > r9d) {
            loc_ADDR:
            if (edi != 0) goto loc_ADDR;
        }
        ecx = *(u32*)(((0xADDR + (rax * 4)) + 0xADDR));
        rcx += rsi;
        __asm("jmp rcx");
    }
    /* jump table: 47 targets, selector=index */
    switch (index) {
        case 0: goto loc_ADDR;
        case 1: goto loc_ADDR;
        case 2: goto loc_ADDR;
        case 3: goto loc_ADDR;
        case 4: goto loc_ADDR;
        case 5: goto loc_ADDR;
        case 6: goto loc_ADDR;
        case 7: goto loc_ADDR;
        case 8: goto loc_ADDR;
        case 9: goto loc_ADDR;
        case 10: goto loc_ADDR;
        case 11: goto loc_ADDR;
        case 12: goto loc_ADDR;
        case 13: goto loc_ADDR;
        case 14: goto loc_ADDR;
        case 15: goto loc_ADDR;
        case 16: goto loc_ADDR;
        case 17: goto loc_ADDR;
        case 18: goto loc_ADDR;
        case 19: goto loc_ADDR;
        case 20: goto loc_ADDR;
        case 21: goto loc_ADDR;
        case 22: goto loc_ADDR;
        case 23: goto loc_ADDR;
        case 24: goto loc_ADDR;
        case 25: goto loc_ADDR;
        case 26: goto loc_ADDR;
        case 27: goto loc_ADDR;
        case 28: goto loc_ADDR;
        case 29: goto loc_ADDR;
        case 30: goto loc_ADDR;
        case 31: goto loc_ADDR;
        case 32: goto loc_ADDR;
        case 33: goto loc_ADDR;
        case 34: goto loc_ADDR;
        case 35: goto loc_ADDR;
        case 36: goto loc_ADDR;
        case 37: goto loc_ADDR;
        case 38: goto loc_ADDR;
        case 39: goto loc_ADDR;
        case 40: goto loc_ADDR;
        case 41: goto loc_ADDR;
        case 42: goto loc_ADDR;
        case 43: goto loc_ADDR;
        case 44: goto loc_ADDR;
        case 45: goto loc_ADDR;
        xmm2 = 0;
        rax = r8;
        return;
    }
    loc_ADDR:
    xmm1 >>= 1;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 2;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 3;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 4;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 5;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 6;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 7;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 8;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 9;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 >>= 0xADDR;
    loc_ADDR:
    xmm1 <<= 1;
    xmm1 >>= 1;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 2;
    xmm1 >>= 2;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 3;
    xmm1 >>= 3;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 4;
    xmm1 >>= 4;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 5;
    xmm1 >>= 5;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 6;
    xmm1 >>= 6;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 7;
    xmm1 >>= 7;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 8;
    xmm1 >>= 8;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 9;
    xmm1 >>= 9;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 0xADDR;
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 0xADDR;
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 0xADDR;
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 0xADDR;
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 0xADDR;
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 <<= 0xADDR;
    xmm1 >>= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm1 = 0;
    return sub_140006f30();
    loc_ADDR:
    xmm2 <<= 1;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 2;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 3;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 4;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 5;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 6;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 7;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 8;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 9;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 0xADDR;
    goto loc_ADDR;
    loc_ADDR:
    xmm2 <<= 0xADDR;
    goto loc_ADDR;
    xmm2 = 0;
}
