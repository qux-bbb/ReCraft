// L1 structure recovery (59 blocks)
void sub_1400030a8_l1(void) {
    rsi = var_190;
    r12 = rdx;
    r13 = var_1a8;
    r15 = r8;
    var_68 = r8;
    rdi = rcx;
    var_78 = rdx;
    var_88 = r13;
    var_60 = 0;
    r14 = r9;
    sub_140004050(r12, r9, rsi);
    var_64 = eax;
    ebx = eax;
    if (eax < -1 /* signed */) {
        loc_ADDR:
        sub_140008860();
    }
    if (eax >= *(u32*)((rsi + 4)) /* signed */) {
        sub_140008860();
        return;
    }
    if (*(u32*)(rdi) != 0xADDR) {
        loc_ADDR:
        rax = *(u64*)((r14 + 8));
        var_c0 = rax;
        var_b8 = rsi;
        if (*(u32*)(rdi) != 0xADDR) {
            loc_ADDR:
            if (*(u32*)((rsi + 0xADDR)) <= 0) {
                sub_14000217c(rcx);
                if (*(u64*)((rax + 0xADDR)) != 0) {
                    sub_140008860();
                    return;
                }
                return;
            }
            if (var_198 != 0) {
                sub_140008860();
                return;
            }
            eax = var_1a0;
            var_38 = r13;
            var_30 = eax;
            var_28 = ebx;
            var_20 = rsi;
            sub_140003598(rdi, r12, r15, r14, rsi, ebx, var_1a0, r13);
        }
        if (*(u32*)((rdi + 0xADDR)) != 4) goto loc_ADDR;
    }
    if (*(u32*)((rdi + 0xADDR)) != 4) goto loc_ADDR;
    eax = *(u32*)((rdi + 0xADDR));
    if (eax != 0xADDR) {
        eax += 0xADDR;
        if (eax > 1) goto loc_ADDR;
    }
    if (*(u64*)((rdi + 0xADDR)) != 0) goto loc_ADDR;
    sub_14000217c(rcx);
    if (*(u64*)((rax + 0xADDR)) == 0) return;
    sub_14000217c(rcx);
    rdi = *(u64*)((rax + 0xADDR));
    sub_14000217c(rcx);
    var_60 = 1;
    r15 = *(u64*)((rax + 0xADDR));
    var_68 = r15;
    sub_140002b70(*(u64*)((rdi + 0xADDR)));
    if (rdi == 0) goto loc_ADDR;
    if (*(u32*)(rdi) == 0xADDR) {
        if (*(u32*)((rdi + 0xADDR)) == 4) {
            eax = *(u32*)((rdi + 0xADDR));
            if (eax == 0xADDR) goto loc_ADDR;
            eax += 0xADDR;
            if (eax <= 1) {
                loc_ADDR:
                if (*(u64*)((rdi + 0xADDR)) == 0) goto loc_ADDR;
            }
        }
    }
    sub_14000217c(rcx);
    if (*(u64*)((rax + 0xADDR)) == 0) goto loc_ADDR;
    sub_14000217c(rcx);
    rbx = *(u64*)((rax + 0xADDR));
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = 0;
    sub_1400040e8(rdi, rbx);
    if (al == 0) {
        sub_1400041d0(rbx);
        if (al == 0) goto loc_ADDR;
        goto loc_ADDR;
    }
    ebx = var_64;
    eax = *(u32*)((rdi + 0xADDR));
    if (eax != 0xADDR) {
        eax += 0xADDR;
        if (eax > 1) goto loc_ADDR;
    }
    if (*(u32*)((rsi + 0xADDR)) <= 0) goto loc_ADDR;
    eax = var_1a0;
    var_28 = eax;
    var_20 = rsi;
    sub_140002864(&var_d8, &var_b8, ebx, r14, rsi, var_1a0);
    xmm1 = var_d8;
    xmm0 = (xmm1 >> 8);
    eax = xmm0;
    var_c8 = xmm1;
    if (eax >= var_f0) goto loc_ADDR;
    r15d = var_d0;
    r9 = xmm1;
    do {
        rax = var_c8;
        rax = *(u64*)(rax);
        rdx = (int64)(*(u32*)((rax + 0xADDR)));
        rcx = (rax + (rax * 4));
        rax = *(u64*)((r9 + 8));
        r8 = (rdx + (rcx * 4));
        xmm0 = *(__m128*)((r8 + rax));
        rcx = (int64)(*(u32*)(((r8 + rax) + 0xADDR)));
        var_b0 = ecx;
        eax = xmm0;
        var_a0 = xmm0;
        if (eax > ebx /* signed */) goto loc_ADDR;
        rax = (xmm0 >> 0xADDR);
        if (ebx <= eax /* signed */) {
            rcx += *(u64*)((r14 + 8));
            r12d = 0;
            xmm0 >>= 8;
            rax = (xmm0 >> 0xADDR);
            var_90 = rax;
            if (eax != 0) {
                while (1) {
                    rax = (r12 + (r12 * 4));
                    xmm0 = *(__m128*)((rcx + (rax * 4)));
                    var_f8 = xmm0;
                    eax = *(u32*)(((rcx + (rax * 4)) + 0xADDR));
                    var_108 = eax;
                    rax = *(u64*)((rdi + 0xADDR));
                    rbx = (int64)(*(u32*)((rax + 0xADDR)));
                    sub_140002b44(rcx);
                    rcx = (rax + 4);
                    rax = *(u64*)((rdi + 0xADDR));
                    rcx += rbx;
                    var_70 = rcx;
                    rbx = (int64)(*(u32*)((rax + 0xADDR)));
                    sub_140002b44(rcx);
                    r13d = *(u32*)((rax + rbx));
                    if (r13d > 0 /* signed */) {
                        rax = var_70;
                        do {
                            rbx = (int64)(*(u32*)(rax));
                            sub_140002b44(rcx);
                            rbx += rax;
                            sub_1400037fc(&var_f8, rbx, *(u64*)((rdi + 0xADDR)));
                            if (eax != 0) goto loc_ADDR;
                            rax = var_70;
                            r13d -= 1;
                            rax += 4;
                            var_70 = rax;
                        } while (r13d > 0 /* signed */);
                    }
                    r12d += 1;
                    if (r12d == var_90) goto loc_ADDR;
                }
                loc_ADDR:
                al = var_198;
                r9 = r14;
                r12 = var_78;
                var_58 = al;
                al = var_60;
                var_50 = al;
                var_48 = var_88;
                eax = var_1a0;
                var_40 = eax;
                var_38 = &var_a0;
                rax = &var_f8;
                var_30 = rbx;
                var_28 = rax;
                var_20 = rsi;
                sub_140002fd0(rdi, r12, var_68);
                ebx = var_64;
                goto loc_ADDR;
                loc_ADDR:
                ebx = var_64;
            }
            r12 = var_78;
        }
        loc_ADDR:
        r15d += 1;
    } while (r15d < var_f0);
    loc_ADDR:
    eax = (*(u32*)(rsi) & 0xADDR);
    if (eax < 0xADDR) {
        sub_14000217c(rcx);
        if (*(u64*)((rax + 0xADDR)) != 0) {
            sub_140008860();
            return;
        }
        return;
    }
    rbx = (int64)(*(u32*)((rsi + 0xADDR)));
    if (ebx == 0) goto loc_ADDR;
    sub_140002b30(rcx);
    rax += rbx;
    if (rax == 0) {
        loc_ADDR:
        if ((*(u8*)((rsi + 0xADDR)) & 4) == 0) {
            sub_14000217c(rcx);
            if (*(u64*)((rax + 0xADDR)) != 0) {
                sub_140008860();
                return;
            }
            return;
        }
        sub_1400026ec(r14, rsi);
        if (al != 0) {
            sub_14000217c(rcx);
            if (*(u64*)((rax + 0xADDR)) != 0) {
                sub_140008860();
                return;
            }
            return;
        }
    }
    if ((*(u8*)((rsi + 0xADDR)) & 4) != 0) {
        sub_14000217c(rcx);
        *(u64*)((rax + 0xADDR)) = rdi;
        sub_14000217c(rcx);
        rcx = var_68;
        *(u64*)((rax + 0xADDR)) = rcx;
        sub_1400087a0(var_68);
        return;
    }
    rbx = (int64)(*(u32*)((rsi + 0xADDR)));
    if (ebx != 0) {
        sub_140002b30(rcx);
        rdx = (rbx + rax);
        goto loc_ADDR;
    }
    edx = 0;
    loc_ADDR:
    sub_1400040e8(rdi, rdx);
    if (al != 0) {
        sub_14000217c(rcx);
        if (*(u64*)((rax + 0xADDR)) != 0) {
            sub_140008860();
            return;
        }
        return;
    }
    sub_14000277c(r12, r14, rsi, &var_88);
    var_50 = var_198;
    ecx = -1;
    var_48 = r14;
    var_40 = 0;
    var_38 = ecx;
    var_30 = ecx;
    var_28 = rsi;
    var_20 = 0;
    sub_14000299c(r12, rdi, var_68, rax, 0, rsi, (ecx | -1), (ecx | -1), 0, r14, var_198);
    sub_14000217c(rcx);
    if (*(u64*)((rax + 0xADDR)) != 0) {
        sub_140008860();
        return;
    }
    return;
    loc_ADDR:
    sub_140001f54(rdi, 1);
    sub_140003b90(&var_a0);
    sub_1400043b8(&var_a0, off_140022230);
    loc_ADDR:
    sub_1400087a0(rcx);
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = rdi;
    sub_14000217c(rcx);
    rcx = var_68;
    *(u64*)((rax + 0xADDR)) = rcx;
    sub_1400087a0(var_68);
}
