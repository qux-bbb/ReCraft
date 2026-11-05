// L1 structure recovery (43 blocks)
void sub_140007570_l1(void) {
// some loops keep labels (an inner loop exits its parent)
    rbp = &var_100;
    r15d = 0;
    r13 = (rcx + (rdx * 8));
    r12d = 0;
    var_c8 = 0;
    rdi = r8;
    rsi = rcx;
    do {
        edx = (uint32)(*(u16*)((rdi + 6)));
        cl = *(u8*)((rdi + 8));
        var_80 = ax;
        eax = (uint32)(*(u16*)((rdi + 2)));
        var_82 = ax;
        al = *(u8*)((rdi + 4));
        var_84 = al;
        var_86 = dx;
        var_88 = cl;
        if (dx != 0) {
            eax = edx;
            if (cl == 1) {
                rax += 2;
                goto loc_ADDR;
            }
            if (cl == 2) {
                rax += 4;
            }
            loc_ADDR:
            var_90 = rax;
            goto loc_ADDR;
        }
        var_90 = r15;
        loc_ADDR:
        var_b8 = 0;
        var_28 = &var_40;
        r9 = &var_48;
        rax = &var_190;
        var_98 = r15;
        xmm0 = 0;
        var_20 = rax;
        var_a0 = xmm0;
        var_b0 = r15;
        var_ba = r15b;
        var_48 = r15d;
        var_40 = 0xADDR;
        sub_1400099b4(rsi, rdx);
        ebx = eax;
        sub_140006420(eax);
        if (ebx != 0) {
            eax = r15d;
            var_48 = r15d;
            var_40 = eax;
            goto loc_ADDR;
        }
        eax = var_40;
        loc_ADDR:
        sub_140008078(&var_80, &var_190, eax, 0xADDR);
        if (var_b9 != r15b) goto loc_ADDR;
        rax = &var_c0;
        var_c0 = r15;
        var_60 = rax;
        rax = &var_80;
        xmm0 = var_60;
        var_70 = rax;
        rax = "0B";
        xmm1 = var_70;
        var_118 = rax;
        var_180 = &var_118;
        rax = (*(u64*)((rdi + 0xADDR)) + 0xADDR);
        var_130 = xmm1;
        var_50 = rax;
        var_60 = ax;
        var_62 = *(u8*)((rdi + 4));
        eax = (uint32)(*(u16*)((rdi + 6)));
        var_d0 = ax;
        al = *(u8*)((rdi + 8));
        var_120 = xmm0;
        var_d2 = al;
        xmm0 = 0;
        var_64 = r15d;
        var_68 = r15w;
        var_70 = xmm0;
        var_d4 = r15d;
        var_d8 = r15w;
        var_e0 = xmm0;
        goto loc_ADDR;
        do {
            r14d = var_64;
            ebx = r15d;
            if (r14d != 0) {
                r15 = var_78;
                do {
                    sub_1400068f0(&var_d0, &var_50, *(u64*)((rdi + 0xADDR)));
                    if (al == 0) goto loc_ADDR;
                    eax = var_d4;
                    rcx = var_180;
                    var_40 = eax;
                    var_f0 = rax;
                    rax = var_e8;
                    var_f8 = rax;
                    eax = (uint32)(var_68);
                    var_100 = rax;
                    var_108 = r15;
                    if (rcx == 0) {
                        sub_1400091d4();
                        return;
                    }
                    rax = *(u64*)(rcx);
                    var_28 = &var_40;
                    rdx = &var_f0;
                    var_20 = rdx;
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, &var_108, rbp, &var_f8, &var_f0, &var_40);
                    if (al == 0) goto loc_ADDR;
                    ebx += 1;
                } while (ebx < r14d);
                loc_ADDR:
                r15d = 0;
            }
            loc_ADDR:
            sub_1400068f0(&var_60, &var_50, *(u64*)((rdi + 0xADDR)));
        } while (al != 0);
        rcx = var_180;
        if (rcx != 0) {
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010();
        }
        bl = 1;
        while (1) {
            if (var_b8 == r15b) goto loc_ADDR;
            r8d = var_a0;
            eax = var_48;
            r8d -= var_98;
            var_30 = 1;
            var_28 = eax;
            sub_140009a5c(rsi, var_98, r8);
            if (eax != 0xADDR) goto loc_ADDR;
            r12 += 1;
            bl = r15b;
            goto loc_ADDR;
            loc_ADDR:
            rcx = var_180;
            r15d = 0;
            if (rcx != 0) {
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010();
            }
            bl = r15b;
        }
        loc_ADDR:
        if (eax != 0) {
            r8d = var_a0;
            r8d -= var_98;
            var_30 = r15d;
            var_28 = r15d;
            sub_140009a5c(rsi, var_98, r8);
        }
        loc_ADDR:
        rax = var_c0;
        rsi += 8;
        var_c8 = rax;
        loc_ADDR:
        r14 = var_b0;
        var_b0 = r15;
        if (r14 != 0) {
            GetProcessHeap();
            HeapFree(rax, 0, r14);
        }
        if (bl != 0) return;
        if (rsi >= r13) return;
    } while (r12 < 0xADDR);
    return;
    loc_ADDR:
    rbx = var_b0;
    var_b0 = r15;
    if (rbx == 0) return;
    GetProcessHeap();
    HeapFree(rax, 0, rbx);
    return;
}
