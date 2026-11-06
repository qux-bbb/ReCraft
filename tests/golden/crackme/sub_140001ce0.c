// L1 structure recovery (37 blocks)
void sub_140001ce0_l1(void) {
    rsi = rcx;
    r15 = r9;
    rbp = r8;
    r13 = rdx;
    sub_140002124(r8);
    r12 = *(u64*)((r15 + 8));
    r14 = *(u64*)(r15);
    rbx = *(u64*)((r15 + 0xADDR));
    r14 -= r12;
    edi = *(u32*)((r15 + 0xADDR));
    if ((*(u8*)((rsi + 4)) & 0xADDR) != 0) {
        rbp = *(u64*)((r15 + 0xADDR));
        rbp -= r12;
        while (1) {
            r8d = *(u32*)(rbx);
            if (edi >= r8d) break;
            r9d = edi;
            r9 += r9;
            eax = *(u32*)(((rbx + (r9 * 8)) + 4));
            if (r14 < rax) {
                edi += 1;
            } else {
                eax = *(u32*)(((rbx + (r9 * 8)) + 8));
                if (r14 < rax) {
                    if ((*(u8*)((rsi + 4)) & 0xADDR) == 0) {
                        eax = edi;
                        rax += 1;
                        ecx = edi;
                        rax += rax;
                        rcx += rcx;
                        if (*(u32*)((rbx + (rax * 8))) == 0) {
                            eax = (rdi + 1);
                            rdx = r13;
                            *(u32*)((r15 + 0xADDR)) = eax;
                            r8d = *(u32*)(((rbx + (rcx * 8)) + 0xADDR));
                            cl = 1;
                            r8 += r12;
                            (*r8)();
                        } else {
                            eax = *(u32*)((rbx + (rax * 8)));
                            if (rbp == rax) {
                                if ((*(u8*)((rsi + 4)) & 0xADDR) != 0) break;
                                edi += 1;
                            }
                            break;
                        }
                        break;
                    }
                    edx = 0;
                    if (r8d != 0) {
                        while (1) {
                            ecx = edx;
                            rcx += rcx;
                            eax = *(u32*)(((rbx + (rcx * 8)) + 4));
                            if (rbp < rax) {
                                edx += 1;
                                if (edx >= r8d) break;
                            } else {
                                eax = *(u32*)(((rbx + (rcx * 8)) + 8));
                                if (rbp < rax) {
                                    eax = *(u32*)(((rbx + (r9 * 8)) + 0xADDR));
                                    if (*(u32*)(((rbx + (rcx * 8)) + 0xADDR)) == eax) {
                                        eax = *(u32*)(((rbx + (r9 * 8)) + 0xADDR));
                                        if (*(u32*)(((rbx + (rcx * 8)) + 0xADDR)) == eax) break;
                                    }
                                    break;
                                }
                                break;
                            }
                            break;
                        }
                    }
                    if (edx != *(u32*)(rbx)) break;
                    break;
                }
                break;
            }
            break;
        }
        eax = 1;
    } else {
        var_30 = rsi;
        var_38 = rbp;
        while (edi < *(u32*)(rbx)) {
            ecx = edi;
            rcx += rcx;
            ebp = edi;
            eax = *(u32*)(((rbx + (rcx * 8)) + 4));
            if (r14 < rax) {
                edi += 1;
            } else {
                eax = *(u32*)(((rbx + (rcx * 8)) + 8));
                if (r14 < rax && *(u32*)(((rbx + (rcx * 8)) + 0xADDR)) != 0) {
                    if (*(u32*)(((rbx + (rcx * 8)) + 0xADDR)) == 1) {
                        if (*(u32*)(rsi) == 0xADDR && qword_ADDR != 0) {
                            sub_140015ab0(off_1400184d0);
                            if (eax != 0) {
                                edx = 1;
                                rcx = rsi;
                                (*qword_ADDR)();
                            }
                        }
                        rax = (rbp + 1);
                        rax += rax;
                        ecx = *(u32*)((rbx + (rax * 8)));
                        rcx += r12;
                        sub_1400020f0(rcx, r13, 1);
                        rax = (rbp + 1);
                        rax += rax;
                        edx = *(u32*)((rbx + (rax * 8)));
                        rax = *(u64*)((r15 + 0xADDR));
                        rdx += r12;
                        var_28 = rax;
                        rax = *(u64*)((r15 + 0xADDR));
                        var_20 = rax;
                        RtlUnwindEx(r13, (rdx + r12), rsi, *(u32*)(rsi), *(u64*)((r15 + 0xADDR)), *(u64*)((r15 + 0xADDR)));
                        sub_140002120();
                    } else {
                        eax = *(u32*)(((rbx + (rcx * 8)) + 0xADDR));
                        rdx = r13;
                        rax += r12;
                        rcx = &var_30;
                        (*rax)();
                        if (eax >= 0 /* signed */) {
                            edi += 1;
                        }
                    }
                }
            }
        }
        eax = 1;
        r11 = &var_40;
        return;
    }
    r11 = &var_40;
    return;
}
