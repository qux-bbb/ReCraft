// L1 structure recovery (65 blocks)
void sub_140004f54_l1(void) {
    r12 = *(u64*)(rdx);
    ebp = r8d;
    rbx = rdx;
    r13 = rcx;
    if (r12 != 0) {
        if (ebp == 0) {
            sil = *(u8*)(r12);
            rcx = (r12 + 1);
            r14d = 0;
            eax = (uint32)(r9b);
            edi = eax;
            *(u64*)(rdx) = rcx;
            r15d = (r14 + 2);
            edi |= r15d;
            edi = ((sil != 0xADDR) ? eax : edi);
            if (sil == 0xADDR) {
                sil = *(u8*)(rcx);
                rcx += 1;
                *(u64*)(rdx) = rcx;
            }
            if ((ebp & 0xADDR) == 0) {
                eax = (rsi - 0xADDR);
                if (al > 9) {
                    eax = (rsi - 0xADDR);
                    if (al > 0xADDR) {
                        eax = (rsi - 0xADDR);
                        if (al > 0xADDR) {
                            eax = 0xADDR;
                            loc_ADDR:
                            eax = ((ebp != 0) ? ebp : eax);
                            ebp = eax;
                        } else {
                            eax = (int32)(sil);
                            eax -= 0xADDR;
                            loc_ADDR:
                            if (eax == 0) {
                                dl = *(u8*)(rcx);
                                r8 = (rcx + 1);
                                *(u64*)(rbx) = r8;
                                eax = (rdx - 0xADDR);
                                if ((al & 0xADDR) == 0) {
                                    sil = *(u8*)(r8);
                                    rax = (r8 + 1);
                                    *(u64*)(rbx) = rax;
                                    eax = 0xADDR;
                                    goto loc_ADDR;
                                }
                                *(u64*)(rbx) = rcx;
                                eax = 8;
                                eax = ((ebp != 0) ? ebp : eax);
                                ebp = eax;
                                if (dl != 0 && *(u8*)(rcx) != dl) {
                                    sub_140008e30();
                                    *(u32*)(rax) = 0xADDR;
                                    sub_140008c88();
                                }
                            }
                        }
                    } else {
                        eax = (int32)(sil);
                        eax -= 0xADDR;
                        goto loc_ADDR;
                    }
                } else {
                    eax = (int32)(sil);
                    eax -= 0xADDR;
                    goto loc_ADDR;
                }
            }
            r8 = *(u64*)(rbx);
            edx = 0;
            __asm("div ebp");
            r9d = -1;
            while (1) {
                ecx = (rsi - 0xADDR);
                if (cl > 9) {
                    eax = (rsi - 0xADDR);
                    if (al > 0xADDR) {
                        eax = (rsi - 0xADDR);
                        if (al > 0xADDR) {
                            ecx = -1;
                        } else {
                            ecx = (int32)(sil);
                            ecx -= 0xADDR;
                        }
                    } else {
                        ecx = (int32)(sil);
                        ecx -= 0xADDR;
                    }
                } else {
                    ecx = (int32)(sil);
                    ecx -= 0xADDR;
                }
                if (ecx >= ebp) break;
                sil = *(u8*)(r8);
                eax = r14d;
                eax *= ebp;
                edx = (rax + rcx);
                ecx = 0;
                cl = (uint8)((edx < eax));
                eax = 0;
                r14d = edx;
                al = (uint8)((r14d > r9d));
                ecx |= eax;
                ecx <<= 2;
                ecx |= 8;
                edi |= ecx;
                r8 += 1;
                *(u64*)(rbx) = r8;
                continue;
            }
            rax = (r8 - 1);
            *(u64*)(rbx) = rax;
            if (sil != 0 && *(u8*)(rax) != sil) {
                sub_140008e30();
                *(u32*)(rax) = 0xADDR;
                sub_140008c88();
            }
            if ((dil & 8) != 0) {
                r8d = 0xADDR;
                r9d = (r8 - 1);
                if ((dil & 4) == 0) {
                    if ((dil & 1) == 0) {
                        if ((r15b & dil) != 0) {
                            r14d = -(r14d);
                        }
                        loc_ADDR:
                        rax = *(u64*)((rbx + 8));
                        if (rax != 0) {
                            *(u64*)(rax) = *(u64*)(rbx);
                        }
                        eax = r14d;
                    } else if ((r15b & dil) == 0) {
                        if (r14d > r9d) {
                            loc_ADDR:
                            eax = edi;
                            loc_ADDR:
                            r15d &= edi;
                            *(u8*)((r13 + 0xADDR)) = 1;
                            *(u32*)((r13 + 0xADDR)) = 0xADDR;
                            if ((eax & 1) != 0) {
                                rdx = *(u64*)((rbx + 8));
                                if (r15d == 0) {
                                    if (rdx != 0) {
                                        *(u64*)(rdx) = *(u64*)(rbx);
                                    }
                                    eax = r9d;
                                } else {
                                    if (rdx != 0) {
                                        *(u64*)(rdx) = *(u64*)(rbx);
                                    }
                                    eax = r8d;
                                }
                            } else {
                                r14d = -1;
                                goto loc_ADDR;
                            }
                        }
                    } else if (r14d > r8d) {
                        goto loc_ADDR;
                    }
                } else {
                    eax = 1;
                    ecx = edi;
                    goto loc_ADDR;
                }
            } else {
                rax = *(u64*)((rbx + 8));
                *(u64*)(rbx) = r12;
                if (rax != 0) {
                    *(u64*)(rax) = r12;
                }
                eax = 0;
            }
        } else {
            eax = (r8 - 2);
            if (eax > 0xADDR) {
                *(u8*)((rcx + 0xADDR)) = 1;
                r9d = 0;
                *(u32*)((rcx + 0xADDR)) = 0xADDR;
                r8d = 0;
                var_28 = r13;
                ecx = 0;
                var_20 = 0;
                edx = 0;
                sub_140008bd0(0, 0, 0, 0, 0, r13);
                rcx = *(u64*)((rbx + 8));
                if (rcx == 0) {
                    eax = 0;
                    return;
                }
                *(u64*)(rcx) = *(u64*)(rbx);
                eax = 0;
                return;
            }
        }
    } else {
        sub_140008e30();
        *(u32*)(rax) = 0xADDR;
        sub_140008c88();
        rcx = *(u64*)((rbx + 8));
        if (rcx == 0) {
            eax = 0;
            return;
        }
        *(u64*)(rcx) = *(u64*)(rbx);
        eax = 0;
        return;
    }
    return;
}
