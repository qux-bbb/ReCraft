// L1 structure recovery (53 blocks)
void sub_14001f5f0_l1(void) {
    r10 = rcx;
    r11d = r8d;
    rsi = rdx;
    *(__m128*)(rcx) = 0;
    *(u64*)((rcx + 0xADDR)) = 0;
    ecx = r8d;
    if (r8d == 0) {
        eax = *(u32*)(rdx);
        r14d = 0;
        edx = (r14 + 4);
        ebx = (rdx - 3);
        r14b = (uint8)((r11d == edx));
        do {
            *(u32*)((r10 + 4)) = 0;
            ecx = eax;
            ecx |= ebx;
            edi = eax;
            r8d = (ecx >> 0xADDR);
            r8d &= ebx;
            if (r8d != r14d) {
                ebp = ((ecx >> 5) & 0xADDR);
                __asm("if (jbe short 0xADDR /* unmodelled flags */) goto 0xADDR");
                r8d = r11d;
                *(u32*)((r10 + 4)) = ebp;
                r8d = -(r8d);
                r9d = ((r8d != 0) ? -1 : 0);
                r9d = ~(r9d);
                r9d &= edx;
                *(u32*)((r10 + 8)) = r9d;
                ecx &= 0xADDR;
                r9d = 0;
                r8d = 0xADDR;
                r9d = ((r11d == edx) ? r8d : r9d);
                r8d = (ecx & 0xADDR);
                ecx = r9d;
                ecx |= r8d;
            }
            r9d = ((ecx >> 5) & 0xADDR);
            r8d = (r9 + 1);
            if (r8d > 0xADDR) {
                r8d = ebx;
                *(u32*)((r10 + 8)) = r11d;
                *(u32*)((r10 + 4)) = r9d;
            }
            r8d <<= 5;
            r8d ^= ecx;
            r8d &= 0xADDR;
            ecx ^= r8d;
            __asm("lock cmpxchg [rsi], ecx");
        } while (*(u32*)(rsi) != eax);
        edi = ~(edi);
        edi &= ebx;
        *(u32*)(r10) = edi;
        *(u32*)((r10 + 0xADDR)) = 0;
    } else {
        ecx -= 1;
        if (ecx == 0) {
            eax = *(u32*)(rdx);
            edi = 0;
            ebx = 1;
            dil = (uint8)((r11d == 5));
            do {
                *(u32*)((r10 + 4)) = 0;
                ecx = eax;
                ecx |= ebx;
                r9d = eax;
                edx = (ecx >> 0xADDR);
                edx &= ebx;
                if (edx != edi) {
                    r8d = ((ecx >> 0xADDR) & 0xADDR);
                    __asm("if (jbe short 0xADDR /* unmodelled flags */) goto 0xADDR");
                    *(u32*)((r10 + 4)) = r8d;
                    edx = 5;
                    edx = ((r11d != ebx) ? ebx : edx);
                    ecx &= 0xADDR;
                    *(u32*)((r10 + 8)) = edx;
                    r8d = 0;
                    edx = 0xADDR;
                    r8d = ((r11d == 5) ? edx : r8d);
                    edx = (ecx & 0xADDR);
                    ecx = r8d;
                    ecx |= edx;
                }
                r8d = ((ecx >> 0xADDR) & 0xADDR);
                edx = (r8 + 1);
                if (edx > 0xADDR) {
                    edx = ebx;
                    *(u32*)((r10 + 8)) = r11d;
                    *(u32*)((r10 + 4)) = r8d;
                }
                edx <<= 0xADDR;
                edx ^= ecx;
                edx &= 0xADDR;
                ecx ^= edx;
                __asm("lock cmpxchg [rsi], ecx");
            } while (*(u32*)(rsi) != eax);
            r9d = ~(r9d);
            r9d &= ebx;
            *(u32*)(r10) = r9d;
            *(u32*)((r10 + 0xADDR)) = 0;
            rax = r10;
            return;
        }
        ecx -= 1;
        if (ecx == 0) {
            edx = 0;
            switch (r11d) {
                case 0xADDR: {
                    r11d -= 0xADDR;
                    edx = 2;
                    break;
                }
                case 0xADDR: {
                    r11d -= 0xADDR;
                    edx = 8;
                    break;
                }
                case 0xADDR: {
                    r11d -= 0xADDR;
                    edx = 4;
                    break;
                }
                case 0xADDR: {
                    r11d -= 0xADDR;
                    edx = (r11 + 0xADDR);
                    break;
                }
                default: {
                    r11d -= 0xADDR;
                    break;
                }
            }
            eax = *(u32*)(rsi);
            ebx = 1;
            do {
                r8d = 0;
                ecx = edx;
                ecx |= eax;
                r9d = eax;
                r8b = (uint8)((ecx == eax));
                *(u32*)((r10 + 0xADDR)) = r8d;
                r8d = ecx;
                r8d |= ebx;
                r8d = ((ecx == eax) ? ecx : r8d);
                __asm("lock cmpxchg [rsi], r8d");
            } while (*(u32*)(rsi) != eax);
            if ((bl & r8b) == 0) {
                ebx = 0;
            } else if ((bl & r9b) != 0) {
                *(u32*)(r10) = 0;
                rax = r10;
                return;
            }
            *(u32*)(r10) = ebx;
        } else {
            ecx -= 1;
            if (ecx != 0) {
                ecx -= 1;
                if (ecx != 0) {
                    ecx -= 1;
                    if (ecx != 0) {
                        ecx -= 1;
                        if (ecx != 0 && ecx != 1) {
                            r8d += 0xADDR;
                            ebx = (rax + 1);
                            if (r8d >= 0xADDR /* signed */) {
                                *(u32*)((r10 + 0xADDR)) = 0;
                                *(u32*)((r10 + 8)) = r11d;
                                *(u32*)((r10 + 4)) = ebx;
                            } else {
                                eax = *(u32*)((rdx + 4));
                                r9d = (r8d << 5);
                                edx = (rbx + 0xADDR);
                                do {
                                    if ((dl & al) == 0) {
                                        ecx = 0;
                                    } else {
                                        ecx = ((eax >> 5) & 0xADDR);
                                        if (ecx == r8d) {
                                            ecx = ebx;
                                        }
                                    }
                                    *(u32*)((r10 + 0xADDR)) = ecx;
                                    ecx = r9d;
                                    ecx ^= eax;
                                    ecx &= 0xADDR;
                                    ecx ^= eax;
                                    ecx |= edx;
                                    __asm("lock cmpxchg [rsi+0xADDR], ecx");
                                } while (*(u32*)((rsi + 4)) != eax);
                                if (*(u32*)((r10 + 0xADDR)) == 0) {
                                    *(u32*)((r10 + 0xADDR)) = 0;
                                    *(u32*)((r10 + 8)) = r11d;
                                    *(u32*)((r10 + 4)) = ebx;
                                    rax = r10;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    rax = r10;
    return;
}
