// L1 structure recovery (31 blocks)
void sub_14000c520_l1(void) {
    sub_14000d990();
    edi = 0;
    if (al != 0) {
        esi = 1;
        rcx = qword_ADDR;
        rcx = ((dword_ADDR != edi) ? qword_ADDR : rcx);
        dword_ADDR = 1;
        sub_14000c7f0(rcx);
        dword_ADDR = edi;
        r12d = eax;
        if (eax == 6) {
            edx = dword_ADDR;
            r15d = esi;
            if (edx != 0) {
                var_28 = rdi;
                rcx = "*.txt";
                rcx = ((edx == 0) ? qword_ADDR : rcx);
                SHStrDupW(((edx == 0) ? qword_ADDR : rcx), &var_28);
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    while (r15d != 0) {
                        var_30 = rdi;
                        var_20 = rdi;
                        sub_14000d6a8(qword_ADDR, var_28, &var_20, &var_30, rdi);
                        ebx = eax;
                        if (eax >= 0 /* signed */) {
                            edx = 0;
                            sub_1400029dc(&var_40, 0, 0xADDR);
                            rcx = var_30;
                            al = dil;
                            if (rcx != 0) {
                                sub_140026734(var_30, &var_40, r8);
                                ebx = eax;
                                if (eax < 0 /* signed */) {
                                    al = dil;
                                } else {
                                    al = sil;
                                }
                            }
                            al = -(al);
                            rax = &var_40;
                            r9 = ((al != 0) ? rax : 0);
                            sub_140011af8(rcx, &var_20, sil, r9);
                            if (al == 0) {
                                sub_140010b30(var_20);
                                r14 = var_28;
                                if (r14 != 0) {
                                    GetLastError();
                                    ebx = eax;
                                    CoTaskMemFree(r14);
                                    SetLastError(ebx);
                                }
                                var_28 = rdi;
                                SHStrDupW(var_20, &var_28);
                                ebx = eax;
                            } else {
                                sub_14000a0c0(0xADDR, &var_20);
                                eax = dword_ADDR;
                                r15d = edi;
                                dword_ADDR = eax;
                                sub_14000fe3c(rcx);
                            }
                        }
                        rcx = var_20;
                        if (rcx != 0) {
                            CoTaskMemFree(var_20);
                        }
                        rcx = var_30;
                        if (rcx != 0) {
                            var_30 = rdi;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010();
                        }
                        if (ebx < 0 /* signed */) break;
                    }
                }
                rcx = var_28;
                if (rcx != 0) {
                    CoTaskMemFree(var_28);
                }
                if (ebx < 0 /* signed */) {
                    r12d = 2;
                    if (ebx == 0xADDR) {
                        MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, 0xADDR);
                    }
                }
            } else {
                ecx = dword_ADDR;
                r9d = 0;
                dword_ADDR = ecx;
                r8d = 0;
                sub_140011af8(dword_ADDR, 0xADDR, 0, 0);
                if (al == 0) {
                    edx = dword_ADDR;
                }
            }
            word_ADDR += si;
        }
        dil = (uint8)((r12d != 2));
        eax = edi;
    } else {
        eax = (rdi + 1);
    }
    r11 = &var_250;
    return;
}
