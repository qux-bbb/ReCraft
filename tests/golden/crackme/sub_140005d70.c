// L1 structure recovery (71 blocks)
void sub_140005d70_l1(void) {
// some loops keep labels (an inner loop exits its parent)
    rax = *(u64*)((rcx + 8));
    rbx = rcx;
    rcx = *(u64*)((rcx + 0xADDR));
    ebp = -1;
    esi = 0;
    if (rcx != 0) {
        sub_140004b38(rcx);
        if (al == 0) {
            eax = ebp;
        } else {
            rcx = *(u64*)((rbx + 0xADDR));
            if (rcx != 0) {
                eax = (*(u32*)((rbx + 0xADDR)) + 1);
                *(u32*)((rbx + 0xADDR)) = eax;
                if (eax == 2) {
                    eax = *(u32*)((rbx + 0xADDR));
                } else {
                    r14 = off_1400194f0;
                    do {
                        *(u32*)((rbx + 0xADDR)) = esi;
                        *(u8*)((rbx + 0xADDR)) = sil;
                        r8b = *(u8*)(rcx);
                        rcx += 1;
                        while (1) {
                            *(u64*)((rbx + 0xADDR)) = rcx;
                            *(u8*)((rbx + 0xADDR)) = r8b;
                            if (r8b == 0) break;
                            if (*(u32*)((rbx + 0xADDR)) < esi /* signed */) break;
                            eax = (r8 - 0xADDR);
                            if (al > 0xADDR) {
                                cl = sil;
                            } else {
                                rax = ((int64)(r8b) - 0xADDR);
                                eax &= 0xADDR;
                                cl = *(u8*)(((r14 + (rax * 2)) + 1));
                            }
                            ecx = (uint32)(cl);
                            rdx = (rax + (rcx * 8));
                            edx &= 0xADDR;
                            eax = (uint32)(*(u8*)((r14 + (rdx * 2))));
                            *(u8*)((rbx + 0xADDR)) = al;
                            if (al < 8) {
                                ecx = eax;
                                if (al == 0) {
                                    rdi = *(u64*)((rbx + 8));
                                    *(u8*)((rbx + 0xADDR)) = sil;
                                    if (*(u8*)((rdi + 0xADDR)) == sil) {
                                        sub_140006cc0(rdi);
                                    }
                                    r8 = (int64)(*(u8*)((rbx + 0xADDR)));
                                    r9 = r8;
                                    if (r8d < ebp /* signed */) {
                                        rax = *(u64*)((rbx + 0xADDR));
                                        rdx = *(u64*)((rbx + 8));
                                        ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                                        if ((cl & 1) == 0) {
                                            ecx = (int32)(r8b);
                                            r8 = rdx;
                                            sub_14000acfc((int32)(r8b), *(u64*)((rbx + 0xADDR)));
                                            if (eax == ebp) {
                                                *(u32*)((rbx + 0xADDR)) = ebp;
                                            } else {
                                                *(u32*)((rbx + 0xADDR)) += 1;
                                            }
                                        } else {
                                            rax = *(u64*)((rbx + 0xADDR));
                                        }
                                        loc_ADDR:
                                        rax = *(u64*)((rbx + 0xADDR));
                                        r8b = *(u8*)(rax);
                                        rcx = (rax + 1);
                                    } else {
                                        rax = *(u64*)((rdi + 0xADDR));
                                        rcx = *(u64*)(rax);
                                        if ((*(u16*)((rcx + (r8 * 2))) & 0xADDR) != 0) {
                                            rax = *(u64*)((rbx + 0xADDR));
                                            r8 = *(u64*)((rbx + 8));
                                            edx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                                            if ((dl & 1) == 0) {
                                                sub_14000acfc(r9d, *(u64*)((rbx + 0xADDR)));
                                                if (eax == ebp) {
                                                    *(u32*)((rbx + 0xADDR)) = ebp;
                                                } else {
                                                    *(u32*)((rbx + 0xADDR)) += 1;
                                                }
                                            } else {
                                                rax = *(u64*)((rbx + 0xADDR));
                                            }
                                            rax = *(u64*)((rbx + 0xADDR));
                                            r8b = *(u8*)(rax);
                                            rax += 1;
                                            *(u64*)((rbx + 0xADDR)) = rax;
                                            *(u8*)((rbx + 0xADDR)) = r8b;
                                        }
                                    }
                                } else {
                                    ecx -= 1;
                                    if (ecx == 0) {
                                        *(u64*)((rbx + 0xADDR)) = rsi;
                                        *(u8*)((rbx + 0xADDR)) = sil;
                                        *(u32*)((rbx + 0xADDR)) = ebp;
                                        *(u32*)((rbx + 0xADDR)) = esi;
                                        *(u8*)((rbx + 0xADDR)) = sil;
                                        goto loc_ADDR;
                                    }
                                    ecx -= 1;
                                    if (ecx == 0) {
                                        if (r8b == 0xADDR) goto loc_ADDR;
                                    } else {
                                        ecx -= 1;
                                        if (ecx == 0) {
                                            if (r8b == 0xADDR) {
                                                rcx = *(u64*)((rbx + 0xADDR));
                                                rax = (rcx + 8);
                                                *(u64*)((rbx + 0xADDR)) = rax;
                                                eax = *(u32*)(rcx);
                                                *(u32*)((rbx + 0xADDR)) = eax;
                                                if (eax < 0 /* signed */) {
                                                    *(u32*)((rbx + 0xADDR)) |= 4;
                                                    eax = -(eax);
                                                    *(u32*)((rbx + 0xADDR)) = eax;
                                                }
                                                loc_ADDR:
                                                al = 1;
                                            } else {
                                                rdx = (rbx + 0xADDR);
                                                sub_140005ce0(rbx, rdx);
                                            }
                                            loc_ADDR:
                                            if (al == 0) {
                                                eax = ebp;
                                                return;
                                            }
                                        } else {
                                            switch (ecx) {
                                                case 0xADDR: {
                                                    ecx -= 0xADDR;
                                                    *(u32*)((rbx + 0xADDR)) = esi;
                                                    goto loc_ADDR;
                                                }
                                                case 0xADDR: {
                                                    ecx -= 0xADDR;
                                                    if (r8b == 0xADDR) goto loc_ADDR;
                                                    break;
                                                }
                                                case 0xADDR: {
                                                    ecx -= 0xADDR;
                                                    sub_1400060f4(rbx);
                                                    goto loc_ADDR;
                                                }
                                                case 0xADDR: {
                                                    ecx -= 0xADDR;
                                                    sub_14000627c(rbx);
                                                    goto loc_ADDR;
                                                }
                                                default: {
                                                    ecx -= 0xADDR;
                                                    eax = ebp;
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        rdx = (rbx + 0xADDR);
                        loc_ADDR:
                        rcx = *(u64*)((rbx + 0xADDR));
                        rax = (rcx + 8);
                        *(u64*)((rbx + 0xADDR)) = rax;
                        eax = *(u32*)(rcx);
                        *(u32*)((rbx + 0xADDR)) = eax;
                        if (eax >= 0 /* signed */) goto loc_ADDR;
                        *(u32*)((rbx + 0xADDR)) = ebp;
                        goto loc_ADDR;
                        if (r8b != 0xADDR) {
                            if (r8b != 0xADDR) {
                                if (r8b != 0xADDR) {
                                    if (r8b != 0xADDR) goto loc_ADDR;
                                    *(u32*)((rbx + 0xADDR)) |= 8;
                                    goto loc_ADDR;
                                }
                                *(u32*)((rbx + 0xADDR)) |= 4;
                                goto loc_ADDR;
                            }
                            *(u32*)((rbx + 0xADDR)) |= 1;
                            goto loc_ADDR;
                        }
                        *(u32*)((rbx + 0xADDR)) |= 0xADDR;
                        goto loc_ADDR;
                        loc_ADDR:
                        *(u32*)((rbx + 0xADDR)) |= 2;
                        goto loc_ADDR;
                        *(u32*)((rbx + 0xADDR)) += 1;
                    } while (*(u32*)((rbx + 0xADDR)) != 2);
                }
            } else {
                rcx = *(u64*)((rbx + 8));
                *(u8*)((rcx + 0xADDR)) = 1;
                *(u32*)((rcx + 0xADDR)) = 0xADDR;
                rcx = *(u64*)((rbx + 8));
                var_28 = rcx;
                r9d = 0;
                var_20 = rsi;
                r8d = 0;
                edx = 0;
                ecx = 0;
                sub_140008bd0(0, 0, 0, 0, rsi);
                eax = ebp;
                return;
            }
        }
    } else {
        *(u8*)((rax + 0xADDR)) = 1;
        *(u32*)((rax + 0xADDR)) = 0xADDR;
        var_28 = rax;
    }
    return;
}
