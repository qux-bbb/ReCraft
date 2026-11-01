// L1 structure recovery (80 blocks)
void sub_14000717c_l1(void) {
    rdi = *(u64*)((rcx + 0xADDR));
    r13d = 0;
    r15 = r8;
    r12 = rdx;
    rbx = rcx;
    if (rdi == 0) {
        al = 0;
    } else {
        eax = (uint32)(*(u16*)((rcx + 2)));
        xmm0 = 0;
        r8 = *(u64*)((rcx + 0xADDR));
        rdi += 0xADDR;
        var_38 = ax;
        al = *(u8*)((rcx + 4));
        var_3a = al;
        var_3c = r13d;
        var_40 = r13w;
        var_48 = xmm0;
        var_c0 = r13b;
        while (1) {
            var_30 = rdi;
            sub_1400068f0(&var_38, &var_30, r8);
            r10d = 0;
            sil = al;
            if (al == 0) break;
            ecx = (uint32)(var_40);
            if (r15 != rcx) {
                eax = (uint32)(var_40);
                ecx = r15d;
                ecx -= eax;
            } else {
                r8 = r15;
                sub_1400274a0(r12, var_50);
                ecx = eax;
                r10d = 0;
            }
            if (ecx >= 0 /* signed */ && ecx != 0) {
                rdi = var_30;
                var_30 = rdi;
                if (*(u64*)((rbx + 0xADDR)) <= r10) {
                    eax = (uint32)(*(u16*)((rbx + 6)));
                    xmm0 = 0;
                    r14d = var_3c;
                    esi = r10d;
                    var_58 = ax;
                    al = *(u8*)((rbx + 8));
                    var_5a = al;
                    var_5c = r10d;
                    var_60 = r10w;
                    var_68 = xmm0;
                    if (r14d != 0) {
                        r13d = 0;
                        do {
                            sub_1400068f0(&var_58, &var_30, *(u64*)((rbx + 0xADDR)));
                            if (al == 0) break;
                            esi += 1;
                        } while (esi < r14d);
                        rdi = var_30;
                    }
                    if (r14d != esi) {
                        al = var_3a;
                        var_3c = esi;
                        if (al != 1) {
                            if (al == 2) {
                                rax = var_48;
                                if (rax == 0) {
                                    _o__errno();
                                    *(u32*)(rax) = 0xADDR;
                                    sub_140002934();
                                } else {
                                    *(u32*)(rax) = esi;
                                }
                            }
                        } else {
                            rax = var_48;
                            if (rax != 0) {
                                *(u16*)(rax) = si;
                            }
                        }
                    }
                } else {
                    rax = *(u64*)((rbx + 0xADDR));
                    edx = 0;
                    rax -= *(u64*)((rbx + 0xADDR));
                    __asm("div qword ptr [rbx+0xADDR]");
                    esi = var_3c;
                    if (rsi > rax && esi != eax) {
                        esi = eax;
                        var_3c = eax;
                        al = var_3a;
                        if (al != 1) {
                            if (al == 2) {
                                rax = var_48;
                                if (rax == 0) {
                                    _o__errno();
                                    *(u32*)(rax) = 0xADDR;
                                    sub_140002934();
                                } else {
                                    *(u32*)(rax) = esi;
                                }
                            }
                        } else {
                            rax = var_48;
                            if (rax != 0) {
                                *(u16*)(rax) = si;
                            }
                        }
                    }
                    eax = esi;
                    rax *= *(u64*)((rbx + 0xADDR));
                    rdi += rax;
                }
                r8 = *(u64*)((rbx + 0xADDR));
            }
        }
        rdi = var_30;
        *(u64*)((rbx + 0xADDR)) = rdi;
        r14b = var_3a;
        rcx = r10;
        if (r13b != 0) {
            rsi = var_48;
            r13d = var_3c;
        } else {
            eax = (uint32)(var_38);
            r13d = 1;
            var_3c = r13d;
            rsi = r10;
            var_40 = r15w;
            var_48 = r10;
            var_50 = r12;
            if (ax != 0) {
                rcx = rax;
            } else {
                ecx = (uint32)(r15w);
                rcx += 2;
            }
            if (r14b != r13b) {
                if (r14b == 2) {
                    rcx += 4;
                }
            } else {
                rcx += 2;
            }
        }
        r8d = var_e8;
        eax = (uint32)(*(u16*)((rbx + 6)));
        dl = *(u8*)((rbx + 8));
        var_5c = r8d;
        r8 = var_e0;
        var_60 = r8w;
        var_58 = ax;
        var_5a = dl;
        var_68 = r10;
        var_70 = r9;
        if (ax == 0) {
            eax = (uint32)(r8w);
            rax += 2;
        }
        if (dl != 1) {
            if (dl == 2) {
                rax += 4;
            }
        } else {
            rax += 2;
        }
        rdx = *(u64*)((rbx + 0xADDR));
        r15 = (rax + rcx);
        r8 = *(u64*)((rbx + 0xADDR));
        rcx = rdx;
        rcx -= r8;
        rax = ((r8 < rdx) ? rcx : 0);
        if (rax < r15) {
            al = 0;
            return;
        }
        rdx -= r15;
        rcx = (r15 + rdi);
        rdx -= rdi;
        r8 -= rdi;
        if (r8 != 0) {
            if (rcx == 0) {
                _o__errno((r15 + rdi), rdx, r8, *(u64*)((rbp + 0xADDR)));
                *(u32*)(rax) = 0xADDR;
                sub_140002934();
            } else if (rdi != 0) {
                if (rdx < r8) {
                    _o__errno((r15 + rdi), rdx, r8, *(u64*)((rbp + 0xADDR)));
                    *(u32*)(rax) = 0xADDR;
                    sub_140002934();
                    return;
                }
                rdx = rdi;
                sub_1400274b8();
            }
            r10d = 0;
        }
        r8 = *(u64*)((rbx + 0xADDR));
        r8 += r15;
        *(u64*)((rbx + 0xADDR)) = r8;
        if (var_c0 != r10b) {
            if (r14b != 0) {
                eax = (r13 + 1);
                if (r13d != eax) {
                    if (r14b != 1) {
                        if (r14b == 2) {
                            if (rsi == 0) {
                                _o__errno();
                                *(u32*)(rax) = 0xADDR;
                                sub_140002934();
                            } else {
                                *(u32*)(rsi) = eax;
                            }
                        }
                    } else if (rsi == 0) {
                        _o__errno();
                        *(u32*)(rax) = 0xADDR;
                        sub_140002934();
                        sub_1400090c8(&var_58, &var_30, *(u64*)((rbx + 0xADDR)));
                        *(u8*)((rbx + 0xADDR)) = 1;
                        al = 1;
                        return;
                    }
                    *(u16*)(rsi) = ax;
                }
            }
        } else {
            sub_1400090c8(&var_38, &var_30, r8);
        }
        sub_1400090c8(&var_58, &var_30, *(u64*)((rbx + 0xADDR)));
        *(u8*)((rbx + 0xADDR)) = 1;
        al = 1;
    }
    return;
}
