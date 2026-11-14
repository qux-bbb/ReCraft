// L1 structure recovery (28 blocks)
void sub_140006b2c_l1(void) {
    rsi = *(u64*)((rcx + 0xADDR));
    rdi = rcx;
    rbx = *(u64*)((rcx + 0xADDR));
    rax = rsi;
    rax -= rbx;
    if (rax >= 0xADDR) {
        ebp = 1;
        r14d = (rbp + 1);
        while (rbx != rsi) {
            r9 = *(u64*)((rbx + 8));
            r10d = *(u32*)(rbx);
            prefetchw(r9);
            eax = *(u32*)(r9);
            do {
                ecx = (eax & 0xADDR);
                __asm("lock cmpxchg [r9], ecx");
            } while (*(u32*)(r9) != eax);
            ecx = eax;
            edx = eax;
            ecx >>= 1;
            ecx &= 0xADDR;
            if (ecx != 0) {
                prefetchw((r9 + 4));
                eax = *(u32*)((r9 + 4));
                do {
                    r8d = eax;
                    r8d |= ecx;
                    __asm("lock cmpxchg [r9+0xADDR], r8d");
                } while (*(u32*)((r9 + 4)) != eax);
                eax = ~(eax);
                ecx &= eax;
            }
            r8 = &var_38;
            if ((bpl & cl) != 0) {
                var_38 = r10d;
                r8 = &var_40;
                var_3c = 0xADDR;
            }
            if ((r14b & cl) != 0) {
                *(u32*)(r8) = r10d;
                *(u32*)((r8 + 4)) = 0xADDR;
                r8 += 8;
            }
            if ((cl & 4) != 0) {
                *(u32*)(r8) = r10d;
                *(u32*)((r8 + 4)) = 0xADDR;
                r8 += 8;
            }
            if (ecx >= 8) {
                *(u32*)(r8) = r10d;
                *(u32*)((r8 + 4)) = 0xADDR;
                r8 += 8;
            }
            ecx = (edx >> 5);
            if ((0xADDR & ecx) != 0) {
                cx &= 0xADDR;
                *(u32*)(r8) = r10d;
                eax = edx;
                *(u16*)((r8 + 6)) = cx;
                eax >>= 0xADDR;
                ax &= bp;
                ax <<= 2;
                *(u16*)((r8 + 4)) = ax;
                r8 += 8;
            }
            eax = (edx >> 0xADDR);
            if ((al & 0xADDR) != 0) {
                edx >>= 0xADDR;
                ax &= 0xADDR;
                dx &= bp;
                *(u32*)(r8) = r10d;
                dx <<= 2;
                dx += bp;
                *(u16*)((r8 + 6)) = ax;
                *(u16*)((r8 + 4)) = dx;
                r8 += 8;
            }
            rax = &var_38;
            r8 -= rax;
            r8 >>= 3;
            if (r8 > 0 /* signed */) {
                sub_140008f18(&var_38, r8);
            }
            rbx += 0xADDR;
            continue;
        }
        *(u64*)((rdi + 0xADDR)) = *(u64*)((rdi + 0xADDR));
        rax = qword_ADDR;
        if (rax != 0) {
            r9d = 0;
            r8d = 0;
            ecx = 0;
            sub_140028010(0, 0xADDR);
        } else {
            rax = qword_ADDR;
            if (rax != 0) {
                r9d = 0;
                r8d = 0;
                ecx = 0;
                sub_140028010(0, 0xADDR);
                r11 = &var_70;
                return;
            }
        }
    }
    r11 = &var_70;
    return;
}
