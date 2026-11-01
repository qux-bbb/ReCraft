// L1 structure recovery (22 blocks)
void sub_140003598_l1(void) {
    var_110 = r8;
    var_108 = rdx;
    rbp = r9;
    r15 = r8;
    r12 = rdx;
    rsi = rcx;
    if (*(u32*)(rcx) == 0xADDR) {
        return;
    }
    sub_14000217c(rcx);
    r13d = var_130;
    r14d = var_128;
    rdi = var_120;
    if (*(u64*)((rax + 0xADDR)) == 0) {
        rax = *(u64*)((rbp + 8));
        var_68 = rax;
        var_60 = rdi;
        if (*(u32*)((rdi + 0xADDR)) <= 0) {
            sub_140008860();
        }
        var_28 = r13d;
        var_20 = rdi;
        sub_140002864(&var_98, &var_60, r14d, rbp, rdi, r13d);
        xmm1 = var_98;
        xmm0 = (xmm1 >> 8);
        eax = xmm0;
        var_70 = xmm1;
        if (eax >= var_b0) {
            return;
        }
        ebx = var_78;
        r9 = xmm1;
        do {
            rax = var_70;
            rax = *(u64*)(rax);
            rdx = (int64)(*(u32*)((rax + 0xADDR)));
            rcx = (rax + (rax * 4));
            rax = *(u64*)((r9 + 8));
            r8 = (rdx + (rcx * 4));
            xmm0 = *(__m128*)((r8 + rax));
            rdx = (int64)(*(u32*)(((r8 + rax) + 0xADDR)));
            var_90 = edx;
            eax = xmm0;
            var_80 = xmm0;
            if (eax <= r14d /* signed */) {
                rax = (xmm0 >> 0xADDR);
                if (r14d <= eax /* signed */) {
                    r15 = (*(u64*)((rbp + 8)) - 0xADDR);
                    xmm0 >>= 8;
                    rax = (xmm0 >> 0xADDR);
                    rcx = (rax + (rax * 4));
                    rdx += (rcx * 4);
                    r15 += rdx;
                    r12 = (int64)(*(u32*)((r15 + 4)));
                    if (r12d == 0) {
                        if ((*(u8*)(r15) & 0xADDR) == 0) {
                            rax = var_138;
                            r9 = rbp;
                            var_58 = 0;
                            var_50 = 1;
                            var_48 = rax;
                            rax = &var_80;
                            var_40 = r13d;
                            var_38 = rax;
                            var_30 = 0;
                            var_28 = r15;
                            var_20 = rdi;
                            sub_140002fd0(rsi, var_108, var_110);
                        }
                    } else {
                        sub_140002b30(rcx);
                        rax += r12;
                        if (rax != 0) {
                            r12 = (int64)(*(u32*)((r15 + 4)));
                            if (r12d == 0) {
                                eax = 0;
                            } else {
                                sub_140002b30(rcx);
                                rax += r12;
                            }
                        }
                    }
                }
            }
            ebx += 1;
        } while (ebx < var_b0);
    }
    EncodePointer(0);
    rbx = rax;
    sub_14000217c(rcx);
    if ((*(u64*)((rax + 0xADDR)) == rbx) || (*(u32*)(rsi) == 0xADDR) || (*(u32*)(rsi) == 0xADDR)) {
        rax = *(u64*)((rbp + 8));
        var_68 = rax;
        var_60 = rdi;
        if (*(u32*)((rdi + 0xADDR)) <= 0) {
            sub_140008860();
        }
        var_28 = r13d;
        var_20 = rdi;
        sub_140002864(&var_98, &var_60, r14d, rbp, rdi, r13d);
        xmm1 = var_98;
        xmm0 = (xmm1 >> 8);
        eax = xmm0;
        var_70 = xmm1;
        if (eax >= var_b0) {
            return;
        }
        return;
    }
    rax = var_138;
    var_38 = r14d;
    var_30 = rax;
    var_28 = r13d;
    var_20 = rdi;
    sub_140002634(rsi, r12, r15, rbp, rdi, r13d, var_138, r14d);
    if (eax != 0) return;
}
