// L1 structure recovery (23 blocks)
void sub_140012308_l1(void) {
    edi = 0;
    rsi = rcx;
    dil = (uint8)((dword_ADDR == 2));
    ecx = 0;
    sub_14000c980(0);
    var_30 = 0;
    var_38 = 0;
    edx = 0;
    sub_1400029dc(&var_40, 0, 0xADDR);
    sub_140010a74(rsi, &var_30);
    r14d = 1;
    if (eax < 0 /* signed */) {
        word_ADDR = (word_ADDR + r14w);
        ecx = r14d;
        sub_14000c980(rcx);
        if (var_30 != 0) {
            CoTaskMemFree(var_30);
        }
        eax = r14d;
    } else {
        sub_140026268(var_30, &var_40, r8, &var_38);
        if (eax >= 0 /* signed */) {
            ecx = var_38;
            dword_ADDR = ecx;
            if (ecx == 0) {
                var_20 = 0xADDR;
                sub_14000c3a4(qword_ADDR, qword_ADDR, qword_ADDR, rsi, 0xADDR);
                CloseHandle(qword_ADDR);
                qword_ADDR = (qword_ADDR | -1);
                dword_ADDR = 0;
                if (var_30 != 0) {
                    CoTaskMemFree(var_30);
                }
                eax = 0;
            } else {
                ecx -= r14d;
                if (ecx != 0) {
                    ecx -= r14d;
                    if (ecx == 0) {
                        word_ADDR = (word_ADDR + r14w);
                        if (dword_ADDR == 0) {
                            sub_14000f610(&var_40);
                        } else if (edi == 0) {
                            sub_1400269b0(&var_40);
                        }
                    } else {
                        ecx -= r14d;
                        if (ecx != 0) {
                            ecx -= r14d;
                            if (ecx != 0 && ecx == r14d) {
                                ecx = (r14 + 4);
                                sub_14000c980(rcx);
                                if (var_30 != 0) {
                                    CoTaskMemFree(var_30);
                                }
                                eax = r14d;
                                r11 = &var_250;
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    r11 = &var_250;
    return;
}
