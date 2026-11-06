// L1 structure recovery (40 blocks)
void sub_14000391c_l1(void) {
    rdi = rcx;
    rsi = r9;
    r15 = r8;
    r14 = rdx;
    sub_140002124(r8);
    sub_14000217c(rcx);
    rbx = var_90;
    edx = 0xADDR;
    ecx = 0xADDR;
    r8d = 0xADDR;
    if (*(u32*)((rax + 0xADDR)) != 0) {
        loc_ADDR:
        if ((*(u8*)((rdi + 4)) & 0xADDR) == 0) {
            if (*(u32*)((rbx + 0xADDR)) != 0) {
                if (*(u32*)(rdi) != 0xADDR) {
                    rax = var_a0;
                    var_38 = rax;
                    eax = var_98;
                    var_30 = eax;
                    al = var_a8;
                    var_28 = al;
                    var_20 = rbx;
                    sub_1400030a8(rdi, r14, r15, rsi, rbx, var_a8, var_98, var_a0);
                    loc_ADDR:
                    eax = 1;
                } else if (*(u32*)((rdi + 0xADDR)) < 3) {
                    rax = var_a0;
                    var_38 = rax;
                    eax = var_98;
                    var_30 = eax;
                    al = var_a8;
                    var_28 = al;
                    var_20 = rbx;
                    sub_1400030a8(rdi, r14, r15, rsi, rbx, var_a8, var_98, var_a0);
                    eax = 1;
                    r11 = &var_50;
                    return;
                } else if (*(u32*)((rdi + 0xADDR)) <= 0xADDR) {
                    rax = var_a0;
                    var_38 = rax;
                    eax = var_98;
                    var_30 = eax;
                    al = var_a8;
                    var_28 = al;
                    var_20 = rbx;
                    sub_1400030a8(rdi, r14, r15, rsi, rbx, var_a8, var_98, var_a0);
                    eax = 1;
                    r11 = &var_50;
                    return;
                }
                rax = *(u64*)((rdi + 0xADDR));
                rbp = (int64)(*(u32*)((rax + 8)));
                if (ebp == 0) {
                    rax = var_a0;
                    var_38 = rax;
                    eax = var_98;
                    var_30 = eax;
                    al = var_a8;
                    var_28 = al;
                    var_20 = rbx;
                    sub_1400030a8(rdi, r14, r15, rsi, rbx, var_a8, var_98, var_a0);
                    eax = 1;
                    r11 = &var_50;
                    return;
                }
                sub_140002b44(rcx);
                r10 = rax;
                r10 += rbp;
                if (r10 == 0) {
                    rax = var_a0;
                    var_38 = rax;
                    eax = var_98;
                    var_30 = eax;
                    al = var_a8;
                    var_28 = al;
                    var_20 = rbx;
                    sub_1400030a8(rdi, r14, r15, rsi, rbx, var_a8, var_98, var_a0);
                    eax = 1;
                    r11 = &var_50;
                    return;
                }
                ecx = (uint32)(var_a8);
                r9 = rsi;
                var_38 = ecx;
                r8 = r15;
                rcx = var_a0;
                rdx = r14;
                var_30 = rcx;
                rax = r10;
                ecx = var_98;
                var_28 = ecx;
                rcx = rdi;
                var_20 = rbx;
                (*qword_ADDR)();
            } else {
                eax = *(u32*)(rbx);
                eax &= ecx;
                if (eax < 0xADDR) {
                    eax = *(u32*)(rbx);
                    eax &= ecx;
                    if (eax < 0xADDR) {
                        eax = 1;
                        r11 = &var_50;
                        return;
                    } else if ((*(u8*)((rbx + 0xADDR)) & 4) == 0) {
                        eax = 1;
                        r11 = &var_50;
                        return;
                    }
                } else {
                    rbp = (int64)(*(u32*)((rbx + 0xADDR)));
                    if (ebp != 0) {
                        sub_140002b30(rcx);
                        rax += rbp;
                        if (rax == 0) {
                            ecx = 0xADDR;
                        }
                    }
                }
            }
            r11 = &var_50;
            return;
        }
        if ((*(u32*)((rbx + 4)) == 0) || (var_98 != 0)) {
            eax = 1;
            r11 = &var_50;
            return;
        }
        if ((*(u8*)((rdi + 4)) & 0xADDR) == 0) {
            sub_140002718(r14, rsi, rbx);
            eax = 1;
            r11 = &var_50;
            return;
        }
        if (*(u32*)(rdi) != r8d) {
            if (*(u32*)(rdi) != edx) {
                sub_140002718(r14, rsi, rbx);
                eax = 1;
                r11 = &var_50;
                return;
            }
            r9d = *(u32*)((rdi + 0xADDR));
            if (r9d < -1 /* signed */) {
                sub_140008860();
            }
            if (r9d >= *(u32*)((rbx + 4)) /* signed */) {
                sub_140008860();
                return;
            }
            rcx = *(u64*)((rdi + 0xADDR));
            sub_140003ec4(rcx, rsi, rbx, r9);
            eax = 1;
            r11 = &var_50;
            return;
        }
        sub_140002cb4(rbx, rsi, *(u64*)((rsi + 0xADDR)));
        if ((eax < -1 /* signed */) || (eax >= *(u32*)((rbx + 4)) /* signed */)) {
            sub_140008860();
            return;
        }
        r9d = eax;
        rcx = r14;
    }
    if (*(u32*)(rdi) == 0xADDR) goto loc_ADDR;
    if (*(u32*)(rdi) != edx) return;
    if (*(u32*)((rdi + 0xADDR)) != 0xADDR) {
        eax = *(u32*)(rbx);
        eax &= ecx;
        if (eax < 0xADDR) goto loc_ADDR;
        if ((*(u8*)((rbx + 0xADDR)) & 1) != 0) goto loc_ADDR;
        sub_140008860();
        return;
    }
    __asm("if (je short 0xADDR /* unmodelled flags */) goto 0xADDR");
    eax = *(u32*)(rbx);
    eax &= ecx;
    if (eax < 0xADDR) goto loc_ADDR;
    if ((*(u8*)((rbx + 0xADDR)) & 1) != 0) goto loc_ADDR;
}
