// L1 structure recovery (74 blocks)
void sub_140021474_l1(void) {
    rbp = &var_a9;
    r9d = (uint32)(*(u16*)(rcx));
    rsi = off_140035a90;
    r13d = 0;
    rbx = rcx;
    var_a8 = r13;
    r12d = 1;
    var_b0 = r13d;
    r15d = 0xADDR;
    if (r9w == 0) {
        rcx = r13;
        rdx = &var_a8;
        do {
            rax = (int64)(*(u32*)(rdx));
            rdx += 4;
            rax += rcx;
            rcx += r15;
            *(u16*)((rsi + (rax * 2))) = r13w;
        } while (rcx <= 0xADDR /* signed */);
    } else {
        r8 = qword_ADDR;
        r10d = (r12 + 0xADDR);
        r14d = r12d;
        while (r9w != r10w) {
            rdx = (int64)(*(u32*)(((rbp + (r14 * 4)) - 1)));
            rbx += 2;
            rcx = (r14 << 8);
            rcx += rdx;
            eax = (rdx + 1);
            *(u32*)(((rbp + (r14 * 4)) - 1)) = eax;
            *(u16*)((rsi + (rcx * 2))) = r9w;
            r9d = (uint32)(*(u16*)(rbx));
            if (r9w == 0) break;
        }
        rdi = (int64)(*(u32*)(((rbp + (r14 * 4)) - 1)));
        if (edi < r15d && edi >= 0 /* signed */) {
            r9d = (uint32)(*(u16*)(rbx));
            if (r9w == r10w) {
                rbx += 2;
                eax = (uint32)(*(u16*)(rbx));
                if (ax == *(u16*)(r8)) {
                    r11 = r13;
                    var_20 = r13;
                    if (dword_ADDR != r13d) {
                        rsi = qword_ADDR;
                        r15 = r13;
                    } else {
                        var_28 = qword_ADDR;
                        var_38 = off_1400291e0;
                        var_40 = &var_28;
                        rax = &var_38;
                        var_a0 = rax;
                        sub_1400107f8(&var_20, &var_30);
                        r11 = var_20;
                        r8 = qword_ADDR;
                        rsi = r11;
                        r15 = r11;
                    }
                    rax = -1;
                    do {
                        rax += 1;
                    } while (*(u16*)((rsi + (rax * 2))) != r13w);
                    eax += edi;
                    if (eax < 0xADDR /* signed */) {
                        rax = (int64)(r12d);
                        rcx = off_140035a90;
                        rax <<= 8;
                        rdx -= rdi;
                        rax += rdi;
                        r8 = rsi;
                        rcx += (rax * 2);
                        sub_140008384(rcx, rdx, rsi);
                        rax = -1;
                        do {
                            rax += 1;
                        } while (*(u16*)((rsi + (rax * 2))) != r13w);
                        r8 = qword_ADDR;
                        eax += edi;
                        *(u32*)(((rbp + (r14 * 4)) - 1)) = eax;
                    }
                    if (r15 != 0) {
                        CoTaskMemFree(r11);
                        r8 = qword_ADDR;
                    }
                    rsi = off_140035a90;
                    r15d = 0xADDR;
                    loc_ADDR:
                    r10d = 0xADDR;
                } else if (ax != *(u16*)((r8 + 2))) {
                    if (ax == *(u16*)((r8 + 4))) {
                        rbx += 2;
                        esi = r13d;
                        if (*(u16*)(rbx) == 0xADDR) {
                            while (1) {
                                rbx += 2;
                                _o_iswdigit((uint32)(*(u16*)(rbx)));
                                if (eax == 0) break;
                                eax = (uint32)(*(u16*)(rbx));
                                esi = (rax + (rsi * 2));
                            }
                        }
                        r9d = dword_ADDR;
                        r9d += esi;
                        esi = 0xADDR;
                        sub_1400083f8(&var_b8, esi, "%", r9);
                        wcsnlen(&var_b8, esi);
                        rsi = off_140035a90;
                        ecx = (rax + rdi);
                        if (ecx < r15d /* signed */) {
                            rax = ((int64)(r12d) << 8);
                            rdx = r15;
                            rax += rdi;
                            rdx -= rdi;
                            sub_140008384((rsi + (rax * 2)), rdx, &var_b8);
                            wcsnlen(&var_b8, 0xADDR);
                            *(u32*)(((rbp + (r14 * 4)) - 1)) = (rdi + rax);
                        }
                        rbx -= 2;
                        loc_ADDR:
                        r8 = qword_ADDR;
                        goto loc_ADDR;
                    } else if (ax != *(u16*)((r8 + 6))) {
                        if (ax == *(u16*)((r8 + 8))) {
                            wcsnlen(off_140034670, r15);
                            r10d = (rdi + rax);
                            if (r10d >= r15d /* signed */) goto loc_ADDR;
                        } else if (ax != *(u16*)((r8 + 0xADDR))) {
                            if (ax == *(u16*)((r8 + 0xADDR))) {
                                wcsnlen(off_140034870, r15);
                                r10d = (rdi + rax);
                                if (r10d >= r15d /* signed */) goto loc_ADDR;
                            } else if (ax != *(u16*)((r8 + 0xADDR))) {
                                if (ax != r10w) {
                                    if (ax == *(u16*)((r8 + 0xADDR))) {
                                        r12d = 1;
                                        r14 = r12;
                                    } else if (ax != *(u16*)((r8 + 0xADDR))) {
                                        if (ax == *(u16*)((r8 + 0xADDR))) {
                                            r12d = 2;
                                        } else if (ax != *(u16*)((r8 + 0xADDR))) {
                                            if (ax == *(u16*)((r8 + 0xADDR))) {
                                                r12d = r13d;
                                                r14 = r13;
                                            }
                                        }
                                    }
                                } else {
                                    edx = (rdi + 1);
                                    if (edx < r15d /* signed */) {
                                        rcx = r14;
                                        *(u32*)(((rbp + (r14 * 4)) - 1)) = edx;
                                        rcx <<= 8;
                                        rcx += rdi;
                                        *(u16*)((rsi + (rcx * 2))) = r10w;
                                    }
                                }
                            }
                        }
                    }
                }
                rbx += 2;
                r9d = (uint32)(*(u16*)(rbx));
            }
        }
    }
    r11 = &var_e0;
    return;
    loc_ADDR:
    *(u16*)(rcx) = r13w;
    goto loc_ADDR;
    r8 -= rdx;
    r9 = off_140034870;
    r9 -= rcx;
    do {
        rax = (rdx + r8);
        if (rax == 0) goto loc_ADDR;
        eax = (uint32)(*(u16*)((rcx + r9)));
        if (ax == 0) goto loc_ADDR;
        *(u16*)(rcx) = ax;
        rcx += 2;
        rdx -= 1;
    } while (rdx != 0);
    loc_ADDR:
    while (1) {
        rax = (rcx - 2);
        rax = ((rdx != 0) ? rcx : rax);
        *(u16*)(rax) = r13w;
        loc_ADDR:
        do {
            *(u32*)(((rbp + (r14 * 4)) - 1)) = r10d;
            r8 = (int64)(eax);
            rdx = r15;
            rax = (r14 << 8);
            rax += rdi;
            rcx = (rsi + (rax * 2));
            rdx -= rdi;
        } while (rdx == 0);
        if (rdx > 0xADDR) goto loc_ADDR;
        if (r8 > 0xADDR) goto loc_ADDR;
        r8 -= rdx;
        r9 = off_140034670;
        r9 -= rcx;
        do {
            rax = (rdx + r8);
            if (rax == 0) goto loc_ADDR;
            eax = (uint32)(*(u16*)((rcx + r9)));
            if (ax == 0) goto loc_ADDR;
            *(u16*)(rcx) = ax;
            rcx += 2;
            rdx -= 1;
        } while (rdx != 0);
    }
}
