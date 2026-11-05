// L1 structure recovery (21 blocks)
void sub_140009508_l1(void) {
    rax = *(u64*)((rcx + 8));
    ebx = 0;
    rax -= *(u64*)(rcx);
    edi = 0;
    r14 = rcx;
    if (rax >= 0xADDR) {
        esi = 0;
        do {
            edx = 0;
            sub_1400029dc(&var_50, 0, 0xADDR);
            var_44 = 0;
            var_28 = &var_40;
            r9 = &var_44;
            rax = &var_50;
            var_40 = 0xADDR;
            var_20 = rax;
            sub_1400099b4(off_14002ad00, rdx);
            ebx = eax;
            if (eax == 0) {
                r9d = var_40;
                rax = 0xADDR;
                __asm("mul r9");
                rdx >>= 3;
                rcx = ((rdx + (rdx * 2)) << 2);
                if (r9 != rcx) {
                    var_40 = 0;
                }
                r8 = *(u64*)(r14);
                rax = 0xADDR;
                ecx = r9d;
                __asm("mul rcx");
                rcx = *(u64*)((r14 + 8));
                rax = 0xADDR;
                r10 = rdx;
                rcx -= r8;
                __asm("mul rcx");
                r10 >>= 3;
                rdx >>= 3;
                rax = (rdx + (rdx * 2));
                rdi = (r8 + (rax * 4));
                while (1) {
                    rdx = &var_50;
                    if (r8 == rdi) break;
                    eax = r10d;
                    rcx = (rax + (rax * 2));
                    rdx += (rcx * 4);
                    rax = &var_50;
                    rcx = &var_50;
                    if (rax == rdx) {
                        eax = r9d;
                        rax += 0xADDR;
                        if (rax <= 0xADDR) {
                            xmm0 = *(f64*)(r8);
                            r9d += 0xADDR;
                            *(f64*)(rdx) = xmm0;
                            r10d += 1;
                            *(u32*)((rdx + 8)) = *(u32*)((r8 + 8));
                            var_40 = r9d;
                        }
                    } else {
                        r11d = *(u32*)(r8);
                        while (1) {
                            if (*(u32*)(rcx) != r11d) {
                                rcx += 0xADDR;
                                if (rcx != rdx) continue;
                            } else {
                                eax = (uint32)(*(u16*)((r8 + 4)));
                                if (*(u16*)((rcx + 4)) == ax) break;
                            }
                        }
                        eax = *(u32*)((r8 + 8));
                        *(u32*)((rcx + 8)) += eax;
                        r9d = var_40;
                    }
                    r8 += 0xADDR;
                }
                eax = var_44;
                var_30 = 1;
                var_28 = eax;
                sub_140009a5c(off_14002ad00, &var_50, r9d);
                edi = eax;
            }
            esi += 1;
            if (edi != 0xADDR) break;
            if (esi >= 0xADDR /* signed */) break;
        } while (ebx == 0);
    }
    ebx = ((ebx == 0) ? edi : ebx);
    eax = ebx;
    r11 = &var_1060;
    return;
}
