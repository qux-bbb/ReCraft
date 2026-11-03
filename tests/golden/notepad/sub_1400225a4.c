// L1 structure recovery (70 blocks)
void sub_1400225a4_l1(void) {
    rbp = &var_b9;
    r15d = 0;
    rax = &var_128;
    var_40 = 0;
    var_38 = rax;
    r13 = rcx;
    var_30 = 0;
    ebx = 0xADDR;
    var_28 = 0xADDR;
    var_20 = 0;
    var_128 = r15;
    RegCreateKeyExW(0xADDR, "Software\\Microsoft\\Notepad\\Autosave", 0, 0, 0, 0xADDR, 0, &var_128, 0);
    rcx = var_128;
    if (eax != 0) {
        if (rcx != 0) {
            RegCloseKey(*(u64*)((rbp + 0xADDR)));
        }
        al = 0;
    } else {
        rax = &hKey;
        var_40 = r15;
        var_38 = rax;
        var_30 = r15;
        var_28 = ebx;
        var_20 = r15d;
        hKey = r15;
        RegCreateKeyExW(*(u64*)((rbp + 0xADDR)), *(u64*)((r13 + 0xADDR)), 0, 0, r15d, ebx, r15, &hKey, r15);
        if (eax != 0) {
            loc_ADDR:
            rcx = hKey;
            if (rcx != 0) {
                RegCloseKey(*(u64*)((rbp + 0xADDR)));
            }
            rcx = var_128;
        } else {
            sub_14000d990();
            r14 = -1;
            r12b = al;
            if (al == 0) {
                sub_140009e44(&var_60, off_14002b210, r14);
                ebx = 2;
            } else {
                var_50 = r15;
                sub_140021e54(&var_50, "%s\\%s.autosave", *(u64*)((r13 + 8)), *(u64*)((r13 + 0xADDR)));
                if (eax >= 0 /* signed */) {
                    var_58 = var_50;
                } else {
                    rcx = var_50;
                    var_58 = r15;
                    if (rcx != 0) {
                        CoTaskMemFree(var_n69);
                    }
                }
                rax = &var_58;
                ebx = 1;
            }
            rcx = *(u64*)(rax);
            var_138 = rcx;
            *(u64*)(rax) = r15;
            if ((bl & 2) != 0) {
                rcx = var_60;
                ebx &= -0xADDR;
                if (rcx != 0) {
                    CoTaskMemFree(var_n59);
                }
            }
            if ((bl & 1) != 0) {
                rcx = var_58;
                if (rcx != 0) {
                    CoTaskMemFree(var_n61);
                }
            }
            rax = var_138;
            if (rax != 0) {
                rdi = r14;
                do {
                    rdi += 1;
                } while (*(u16*)((rax + (rdi * 2))) != r15w);
                var_50 = r15;
                var_70 = off_140029230;
                rax = &var_70;
                var_d8 = rax;
                sub_1400107f8(&var_50, &var_68);
                if (eax >= 0 /* signed */) {
                    rbx = var_50;
                    rax = r14;
                    do {
                        rax += 1;
                    } while (*(u16*)((rbx + (rax * 2))) != r15w);
                    rdi += rax;
                    edx = 0;
                    sub_140009e44(&var_60, 0, (rdi + 4));
                    rax = var_138;
                    rdx = (rdi + 4);
                    rdi = var_60;
                    var_28 = rbx;
                    var_20 = rax;
                    sub_1400083f8(rdi, (rdi + 4), "%i,%s,%s", dword_ADDR, var_138, rbx);
                    if (eax < 0 /* signed */) {
                        loc_ADDR:
                        if (rdi != 0) {
                            CoTaskMemFree(rdi);
                        }
                        if (rbx != 0) {
                            rcx = rbx;
                            CoTaskMemFree(rcx);
                        }
                        loc_ADDR:
                        rcx = var_138;
                        if (rcx == 0) goto loc_ADDR;
                    } else {
                        rcx = qword_ADDR;
                        rdx = "*Untitled*";
                        var_50 = r15;
                        rsi = r15;
                        if (rcx == 0) {
                            eax = 0;
                            do {
                                r14 += 1;
                            } while (*(u16*)((rdi + (r14 * 2))) != ax);
                            eax = ((r14 * 2) + 1);
                            var_28 = eax;
                            var_20 = rdi;
                            RegSetValueExW(hKey, rdx, 0, 1);
                            if (eax == 0) {
                                if (r12b == 0) {
                                    if (rsi != 0) {
                                        CoTaskMemFree(rsi);
                                    }
                                    if (rdi != 0) {
                                        CoTaskMemFree(rdi);
                                    }
                                    if (rbx != 0) {
                                        CoTaskMemFree(rbx);
                                    }
                                    rcx = var_138;
                                    if (rcx != 0) {
                                        CoTaskMemFree(*(u64*)((rbp + 0xADDR)));
                                    }
                                    rcx = hKey;
                                    if (rcx != 0) {
                                        RegCloseKey(*(u64*)((rbp + 0xADDR)));
                                    }
                                    rcx = var_128;
                                    if (rcx != 0) {
                                        RegCloseKey(*(u64*)((rbp + 0xADDR)));
                                    }
                                    al = 1;
                                } else {
                                    CreateDirectoryW(*(u64*)((r13 + 8)), 0);
                                    if (eax != 0) {
                                        r15d = dword_ADDR;
                                        r14 = qword_ADDR;
                                        r9d = 0;
                                        dword_ADDR = 2;
                                        sub_140011af8(rcx, &var_138, 1, 0);
                                        if (al != 0) {
                                            qword_ADDR = r14;
                                            dword_ADDR = r15d;
                                        } else {
                                            RegDeleteKeyExW(var_128, *(u64*)((r13 + 0xADDR)), 0, 0);
                                            loc_ADDR:
                                            __asm("if (je short 0xADDR /* unmodelled flags */) goto 0xADDR");
                                            rcx = rsi;
                                            CoTaskMemFree(rcx);
                                        }
                                    } else {
                                        GetLastError();
                                    }
                                }
                            } else {
                                goto loc_ADDR;
                            }
                        } else {
                            sub_140010a74(rcx, &var_50);
                            if (eax >= 0 /* signed */) {
                                rsi = var_50;
                                rdx = rsi;
                                r15 = rsi;
                            } else {
                                rcx = var_50;
                                if (rcx == 0) goto loc_ADDR;
                            }
                        }
                    }
                } else {
                    rcx = var_50;
                    if (rcx == 0) goto loc_ADDR;
                }
            }
        }
    }
    return;
}
