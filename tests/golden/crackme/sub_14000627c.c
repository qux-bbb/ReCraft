// L1 structure recovery (123 blocks)
void sub_14000627c_l1(void) {
    var_40 = -0xADDR;
    rbx = rcx;
    ecx = (int32)(*(u8*)((rcx + 0xADDR)));
    esi = 0xADDR;
    if (ecx > 0xADDR /* signed */) {
        if (ecx > 0xADDR /* signed */) {
            if (ecx == 0xADDR) {
                *(u32*)((rbx + 0xADDR)) = 0xADDR;
                *(u32*)((rbx + 0xADDR)) = 0xADDR;
                dl = 1;
                loc_ADDR:
                sub_140005610(rbx, rdx);
                loc_ADDR:
                r13d = 0;
                if (al != 0) {
                    if (*(u8*)((rbx + 0xADDR)) == 0xADDR) {
                        var_38 = ax;
                        var_3a = al;
                        rdi = 0;
                        edx = *(u32*)((rbx + 0xADDR));
                        eax = (edx >> 4);
                        if ((al & 1) != 0) {
                            eax = (edx >> 6);
                            if ((al & 1) == 0) {
                                if ((*(u8*)((rbx + 0xADDR)) & 1) == 0) {
                                    eax = (edx >> 1);
                                    if ((al & 1) != 0) {
                                        var_38 = 0xADDR;
                                        edi = 1;
                                    }
                                } else {
                                    var_38 = 0xADDR;
                                    edi = 1;
                                }
                            } else {
                                var_38 = 0xADDR;
                                edi = 1;
                            }
                        }
                        cl = *(u8*)((rbx + 0xADDR));
                        eax = (rcx - 0xADDR);
                        if ((al & 0xADDR) != 0) {
                            r8b = r13b;
                        } else {
                            eax = (edx >> 5);
                            r8b = 1;
                        }
                        eax = (rcx - 0xADDR);
                        al = (uint8)(((al & 0xADDR) == 0));
                        if (r8b != 0) {
                            *(u8*)(((rbp + rdi) - 0xADDR)) = 0xADDR;
                            if (cl == 0xADDR) {
                                sil = 0xADDR;
                            }
                            *(u8*)(((rbp + rdi) - 0xADDR)) = sil;
                            rdi += 2;
                        }
                        r15d = *(u32*)((rbx + 0xADDR));
                        r15d -= *(u32*)((rbx + 0xADDR));
                        r15d -= edi;
                        if ((dl & 0xADDR) == 0) {
                            r14 = *(u64*)((rbx + 8));
                            esi = 0;
                            if (r15d > 0 /* signed */) {
                                while (1) {
                                    rax = *(u64*)((rbx + 0xADDR));
                                    ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                                    if ((cl & 1) == 0) {
                                        r8 = r14;
                                        sub_14000acfc(0xADDR, *(u64*)((rbx + 0xADDR)));
                                        if (eax == -1) break;
                                        ecx = *(u32*)((rbx + 0xADDR));
                                        eax = (rcx + 1);
                                        *(u32*)((rbx + 0xADDR)) = eax;
                                        if (ecx != -0xADDR) {
                                            esi += 1;
                                            if (esi < r15d /* signed */) continue;
                                        }
                                    } else {
                                        rax = *(u64*)((rbx + 0xADDR));
                                    }
                                }
                                *(u32*)((rbx + 0xADDR)) = -1;
                            }
                        }
                        r14 = (rbx + 0xADDR);
                        rdx = *(u64*)((rbx + 8));
                        rsi = (rbx + 0xADDR);
                        rax = *(u64*)(r14);
                        ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                        if ((cl & 1) == 0) {
                            var_20 = rdx;
                            sub_140006d38(r14, &var_38, edi, rsi, rdx);
                        } else {
                            rax = *(u64*)(r14);
                            if (*(u64*)((rax + 8)) == r13) {
                                *(u32*)(rsi) += edi;
                            }
                        }
                        ecx = *(u32*)((rbx + 0xADDR));
                        eax = (ecx >> 3);
                        if ((al & 1) != 0) {
                            ecx >>= 2;
                            if ((cl & 1) == 0) {
                                r12 = *(u64*)((rbx + 8));
                                edi = r13d;
                                if (r15d > 0 /* signed */) {
                                    while (1) {
                                        rax = *(u64*)(r14);
                                        ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                                        if ((cl & 1) == 0) {
                                            r8 = r12;
                                            sub_14000acfc(0xADDR, *(u64*)(r14));
                                            if (eax == -1) break;
                                            ecx = *(u32*)(rsi);
                                            eax = (rcx + 1);
                                            *(u32*)(rsi) = eax;
                                            if (ecx != -0xADDR) {
                                                edi += 1;
                                                if (edi < r15d /* signed */) continue;
                                            }
                                        } else {
                                            rax = *(u64*)(r14);
                                        }
                                    }
                                    *(u32*)(rsi) = -1;
                                }
                            }
                        }
                        if (*(u8*)((rbx + 0xADDR)) == r13b) {
                            r10 = (rbx + 0xADDR);
                            rdx = *(u64*)((rbx + 8));
                            r9 = (rbx + 0xADDR);
                            r8d = *(u32*)((rbx + 0xADDR));
                            r11 = *(u64*)((rbx + 0xADDR));
                            rax = *(u64*)(r10);
                            ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                            if ((cl & 1) == 0) {
                                var_20 = rdx;
                                sub_140006d38(r10, r11, r8, r9, rdx);
                            } else {
                                rax = *(u64*)(r10);
                                if (*(u64*)((rax + 8)) == r13) {
                                    *(u32*)(r9) += r8d;
                                }
                            }
                        } else if (*(u32*)((rbx + 0xADDR)) > r13d /* signed */) {
                            rdi = *(u64*)((rbx + 8));
                            if (*(u8*)((rdi + 0xADDR)) == r13b) {
                                sub_140006cc0(rdi);
                            }
                            r12 = *(u64*)((rbx + 0xADDR));
                            ecx = *(u32*)((rbx + 0xADDR));
                            rax = *(u64*)((rdi + 0xADDR));
                            if (*(u32*)((rax + 0xADDR)) != 0xADDR) {
                                edi = r13d;
                                if (ecx != 0) {
                                    r14 = (rbx + 0xADDR);
                                    while (1) {
                                        var_38 = r13d;
                                        r9d = (uint32)(*(u16*)(r12));
                                        r12 += 2;
                                        rax = *(u64*)((rbx + 8));
                                        var_20 = rax;
                                        sub_14000c348(&var_38, &var_48, 6, (uint32)(*(u16*)(r12)), *(u64*)((rbx + 8)));
                                        if (eax != 0) break;
                                        r8d = var_38;
                                        if (r8d == 0) break;
                                        rdx = *(u64*)((rbx + 8));
                                        rax = *(u64*)(r14);
                                        ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                                        if ((cl & 1) == 0) {
                                            var_20 = rdx;
                                            sub_140006d38(r14, &var_48, var_n28, (rbx + 0xADDR), rdx);
                                        } else {
                                            rax = *(u64*)(r14);
                                            if (*(u64*)((rax + 8)) == r13) {
                                                *(u32*)((rbx + 0xADDR)) += r8d;
                                            }
                                        }
                                        edi += 1;
                                        if (edi != *(u32*)((rbx + 0xADDR))) continue;
                                    }
                                    *(u32*)((rbx + 0xADDR)) = -1;
                                }
                            } else {
                                var_38 = r13;
                                esi = r13d;
                                if (ecx != 0) {
                                    rdi = (rbx + 0xADDR);
                                    r14 = (rbx + 0xADDR);
                                    do {
                                        edx = (uint32)(*(u16*)(r12));
                                        r12 += 2;
                                        sub_14000c868(&var_48, (uint32)(*(u16*)(r12)), &var_38, *(u64*)((rbx + 8)));
                                        r8 = rax;
                                        if (rax == -1) {
                                            *(u32*)(rdi) = -1;
                                            break;
                                        }
                                        rdx = *(u64*)((rbx + 8));
                                        rax = *(u64*)(r14);
                                        ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                                        if ((cl & 1) == 0) {
                                            var_20 = rdx;
                                            sub_140006d38(r14, &var_48, r8, rdi, rdx);
                                        } else {
                                            rax = *(u64*)(r14);
                                            if (*(u64*)((rax + 8)) == r13) {
                                                *(u32*)(rdi) += r8d;
                                            }
                                        }
                                        esi += 1;
                                    } while (esi != *(u32*)((rbx + 0xADDR)));
                                }
                            }
                        }
                        if (*(u32*)((rbx + 0xADDR)) >= r13d /* signed */) {
                            eax = (*(u32*)((rbx + 0xADDR)) >> 2);
                            if ((al & 1) != 0) {
                                rsi = *(u64*)((rbx + 8));
                                edi = r13d;
                                if (r15d > 0 /* signed */) {
                                    while (1) {
                                        rax = *(u64*)((rbx + 0xADDR));
                                        ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                                        if ((cl & 1) == 0) {
                                            r8 = rsi;
                                            sub_14000acfc(0xADDR, *(u64*)((rbx + 0xADDR)));
                                            if (eax == -1) break;
                                            ecx = *(u32*)((rbx + 0xADDR));
                                            eax = (rcx + 1);
                                            *(u32*)((rbx + 0xADDR)) = eax;
                                            if (ecx != -0xADDR) {
                                                edi += 1;
                                                if (edi < r15d /* signed */) continue;
                                            }
                                        } else {
                                            rax = *(u64*)((rbx + 0xADDR));
                                        }
                                    }
                                    *(u32*)((rbx + 0xADDR)) = -1;
                                }
                            }
                        }
                    }
                    al = 1;
                } else {
                    al = 0;
                }
            } else if (ecx == 0xADDR) {
                sub_140006c18(rbx);
                goto loc_ADDR;
            } else if (ecx == 0xADDR) {
                edx = 0;
                sub_14000540c(rbx, 0);
                goto loc_ADDR;
            } else if (ecx != esi) {
                al = 0;
                r11 = &var_60;
                return;
            }
            edx = 0;
            goto loc_ADDR;
        } else if (ecx == 0xADDR) {
            ecx = *(u32*)((rbx + 0xADDR));
            eax = (ecx >> 5);
            if ((al & 1) != 0) {
                ecx |= 0xADDR;
                *(u32*)((rbx + 0xADDR)) = ecx;
            }
            edx = 0;
            sub_140005208(rbx, 0);
            goto loc_ADDR;
        }
        ecx -= 0xADDR;
        if (ecx == 0) {
            sub_140006830(rbx);
            goto loc_ADDR;
        }
        ecx -= 1;
        if (ecx != 0) {
            ecx -= 1;
            if (ecx != 0) {
                ecx -= 2;
                if (ecx == 0) {
                    *(u32*)((rbx + 0xADDR)) |= 0xADDR;
                } else if (ecx != 5) {
                    al = 0;
                    r11 = &var_60;
                    return;
                }
                sub_140006b60(rbx);
                goto loc_ADDR;
            }
        }
    } else if (ecx != 0xADDR) {
        if (ecx > 0xADDR /* signed */) {
            if (ecx != 0xADDR && ecx == 0xADDR) {
                sub_1400067b4(rbx);
                goto loc_ADDR;
            } else if (ecx != 0xADDR) {
                if (ecx != 0xADDR) {
                    al = 0;
                    r11 = &var_60;
                    return;
                }
                edx = 0;
                sub_140006a8c(rbx);
                goto loc_ADDR;
            }
        } else if (ecx != 0xADDR) {
            ecx -= 0xADDR;
            if (ecx != 0) {
                ecx -= 2;
                if (ecx != 0) {
                    ecx -= 2;
                    if (ecx != 0) {
                        ecx -= 1;
                        if (ecx != 0 && ecx != 1) {
                            al = 0;
                            r11 = &var_60;
                            return;
                        }
                    }
                }
            }
        }
    }
    r11 = &var_60;
    return;
}
