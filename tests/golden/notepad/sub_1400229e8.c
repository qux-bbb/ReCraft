// L1 structure recovery (64 blocks)
void sub_1400229e8_l1(void) {
    var_e0 = rcx;
    rbp = &var_79;
    r12d = 0;
    rax = &var_68;
    rbx = rcx;
    var_e8 = 0;
    edi = 0xADDR;
    var_68 = 0;
    var_20 = rax;
    RegOpenKeyExW(0xADDR, "Software\\Microsoft\\Notepad\\Autosave", 0, edi, &var_68);
    rcx = var_68;
    if (eax != 0) {
        if (rcx != 0) {
            RegCloseKey(rcx);
        }
        al = 0;
    } else {
        rax = &var_70;
        var_20 = rax;
        var_70 = r12;
        RegOpenKeyExW(*(u64*)((rbp - 0xADDR)), *(u64*)((rbx + 0xADDR)), 0, edi, &var_70);
        rcx = var_70;
        if (eax != 0) {
            if (rcx != 0) {
                RegCloseKey(var_n9);
            }
            rcx = var_68;
        } else {
            var_58 = r12;
            rax = &var_f8;
            var_50 = r12;
            var_48 = rax;
            rax = &var_f0;
            var_64 = r12d;
            var_40 = rax;
            rax = &var_64;
            var_f0 = r12d;
            var_38 = rax;
            var_30 = r12;
            var_28 = r12;
            var_20 = r12;
            var_f8 = r12d;
            RegQueryInfoKeyW(var_n9, 0, 0, 0, r12, r12, r12, &var_64, &var_f0);
            if (eax != 0) {
                loc_ADDR:
                rcx = var_70;
            } else {
                eax = var_f0;
                edi = var_f8;
                eax += 1;
                edx = 0;
                var_f0 = eax;
                rdi >>= 1;
                sub_140009e44(&var_80, 0, eax);
                edx = 0;
                sub_140009e44(&var_e8, 0, (rdi + 1));
                rsi = var_e8;
                rax = &var_f8;
                rbx = var_80;
                var_38 = rax;
                var_30 = rsi;
                var_28 = r12;
                var_20 = r12;
                RegEnumValueW(var_70, 0, rbx, &var_f0, r12, r12, var_e8, &var_f8);
                if (eax == 0) {
                    rdi = (rsi + (rdi * 2));
                    *(u16*)(rdi) = r12w;
                    eax = (uint32)(*(u16*)(rsi));
                    eax -= 0xADDR;
                    var_60 = eax;
                    if (eax > 6) {
                        CoTaskMemFree(rsi);
                        loc_ADDR:
                        if (rbx == 0) goto loc_ADDR;
                    } else {
                        r14d = 0xADDR;
                        wcschr(rsi, r14d);
                        r15 = rax;
                        if (rax != 0) {
                            wcschr((rax + 2), r14d);
                            var_78 = r12;
                            rdx = (r15 + 2);
                            var_e8 = r12;
                            r14 = r12;
                            r13 = rax;
                            r12d = 2;
                            if (rax == 0) {
                                rdi -= r15;
                                rdi -= r12;
                                rdi >>= 1;
                                sub_140009e44(&var_e8, rdx, rdi);
                                sub_14000a0c0(&var_78, rax);
                                rcx = var_e8;
                                r13d = 0;
                                if (rcx != 0) {
                                    CoTaskMemFree(*(u64*)((rbp + 0xADDR)));
                                }
                            } else {
                                rdi -= rax;
                                r8 = rax;
                                rdi -= r12;
                                r8 -= r15;
                                rdi >>= 1;
                                r8 -= r12;
                                r8 >>= 1;
                                sub_140009e44(&var_88, rdx, r8);
                                sub_14000a0c0(&var_78, rax);
                                rcx = var_88;
                                if (rcx != 0) {
                                    CoTaskMemFree(*(u64*)((rbp + 0xADDR)));
                                }
                                sub_140009e44(&var_88, (r13 + 2), rdi);
                                sub_14000a0c0(&var_e8, rax);
                                rcx = var_88;
                                r13d = 0;
                                if (rcx != 0) {
                                    CoTaskMemFree(*(u64*)((rbp + 0xADDR)));
                                }
                                r14 = var_e8;
                            }
                            rdx = rbx;
                            sub_140027494("*Untitled*");
                            if (eax == 0) {
                                r12b = 1;
                            } else {
                                rax = qword_ADDR;
                                rdx = rbx;
                                rdx -= rax;
                                do {
                                    ecx = (uint32)(*(u16*)(rax));
                                    r8d = (uint32)(*(u16*)((rax + rdx)));
                                    ecx -= r8d;
                                    if (ecx != 0) break;
                                    rax += r12;
                                } while (r8d != 0);
                                r12b = r13b;
                            }
                            rdi = var_78;
                            r15d = (uint32)(*(u16*)(rdi));
                            var_e8 = (uint8)((r15w != 0));
                            if (r14 != 0 && *(u16*)(r14) != r13w) {
                                SetCurrentDirectoryW(r14);
                            }
                            if (r15w != 0) {
                                rcx = rdi;
                                rdx = &var_78;
                                var_30 = r13;
                                var_28 = 0xADDR;
                                var_20 = 3;
                                rax = &var_80;
                                rax = ((r15w != 0) ? rdx : rax);
                                var_88 = rax;
                                CreateFileW(rcx, 0xADDR, 3, 0, 3, 0xADDR, r13);
                                qword_ADDR = rax;
                                r13d = 2;
                                if (rax == -1) {
                                    sub_140010b30(rdi);
                                } else {
                                    edx = var_60;
                                    edx = ((r15w != 0) ? r13d : edx);
                                    sub_140010f48(var_88, rdx, var_e8);
                                    rbx = var_80;
                                    rdi = var_78;
                                }
                                if (r15w != 0) {
                                    sub_14000a0c0(&var_78, off_140036148);
                                    ecx = 0;
                                    eax = (uint32)(r12b);
                                    dword_ADDR = eax;
                                    if (r12b == 0) {
                                        var_e8 = rbx;
                                        rdx = &var_e8;
                                        rbx = rcx;
                                        edi = r13d;
                                    } else {
                                        var_80 = rcx;
                                        rdx = &var_80;
                                        edi = (rcx + 1);
                                    }
                                    sub_14000a0c0(off_140036148, rdx);
                                    if ((r13b & dil) != 0) {
                                        rcx = var_e8;
                                        edi &= -0xADDR;
                                        if (rcx != 0) {
                                            CoTaskMemFree(*(u64*)((rbp + 0xADDR)));
                                        }
                                    }
                                    if ((dil & 1) != 0) {
                                        rcx = var_80;
                                        if (rcx != 0) {
                                            CoTaskMemFree(*(u64*)((rbp + 7)));
                                        }
                                    }
                                    eax = var_60;
                                    rcx = qword_ADDR;
                                    dword_ADDR = eax;
                                    SendMessageW(qword_ADDR, 0xADDR, 1, 0);
                                    sub_14000fe3c(rcx);
                                    sub_14000f9f0();
                                    rdi = var_78;
                                    DeleteFileW(rdi);
                                }
                            } else if (r12b == 0) {
                                rcx = rbx;
                                rdx = &var_78;
                                var_30 = r13;
                                var_28 = 0xADDR;
                                var_20 = 3;
                                rax = &var_80;
                                rax = ((r15w != 0) ? rdx : rax);
                                var_88 = rax;
                                CreateFileW(rcx, 0xADDR, 3, 0, 3, 0xADDR, r13);
                                qword_ADDR = rax;
                                r13d = 2;
                                if (rax == -1) {
                                    sub_140010b30(rdi);
                                } else {
                                    edx = var_60;
                                    edx = ((r15w != 0) ? r13d : edx);
                                    sub_140010f48(var_88, rdx, var_e8);
                                    rbx = var_80;
                                    rdi = var_78;
                                }
                                return;
                            }
                            rdx = var_e0;
                            rdx = *(u64*)((rdx + 0xADDR));
                            RegDeleteKeyExW(var_68, *(u64*)((rdx + 0xADDR)), 0, 0);
                            if (r14 != 0) {
                                CoTaskMemFree(r14);
                            }
                            if (rdi != 0) {
                                CoTaskMemFree(rdi);
                            }
                            CoTaskMemFree(rsi);
                            if (rbx != 0) {
                                CoTaskMemFree(rbx);
                            }
                            rcx = var_70;
                            if (rcx != 0) {
                                RegCloseKey(var_n9);
                            }
                            rcx = var_68;
                            if (rcx != 0) {
                                RegCloseKey(var_n11);
                            }
                            al = 1;
                        }
                    }
                } else {
                    if (rsi == 0) goto loc_ADDR;
                }
            }
        }
    }
    return;
}
