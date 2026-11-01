// L1 structure recovery (17 blocks)
void sub_14000ca10_l1(void) {
    var_30 = 0;
    r9d = 0;
    ebx = dword_ADDR;
    var_28 = 0xADDR;
    var_20 = 3;
    CreateFileW(rcx, 0xADDR, (r9 + 1), 0, 3, 0xADDR, 0);
    rsi = rax;
    if (rax != -1) {
        var_40 = 0;
        var_20 = 0;
        ReadFile(rax, &var_50, 0xADDR, &var_40, 0);
        if (eax > 0 /* signed */) {
            edi = var_40;
            if (edi != 0) {
                ebx = dword_ADDR;
                if (edi > 1) {
                    eax = (uint32)(var_50);
                    if (eax == 0xADDR) {
                        if (edi > 2) {
                            eax = 4;
                            ebx = ((var_52 == 0xADDR) ? eax : ebx);
                        }
                    } else if (eax == 0xADDR) {
                        ebx = 2;
                    } else if (eax == 0xADDR) {
                        ebx = 3;
                    } else {
                        var_44 = -1;
                        IsTextUnicode(&var_50, edi, &var_44);
                        if (eax == 0) {
                            sub_140023ae0(&var_50, edi);
                            eax = -(eax);
                            ebx = (((eax != 0) ? 4 : 0) + 1);
                        } else if (var_44 != 2) {
                            ebx = 2;
                            CloseHandle(rsi);
                            eax = ebx;
                            r11 = &var_460;
                            return;
                        } else if (edi < 0xADDR /* signed */) {
                            sub_140023ae0(&var_50, edi);
                            eax = -(eax);
                            ebx = (((eax != 0) ? 4 : 0) + 1);
                            CloseHandle(rsi);
                            eax = ebx;
                            r11 = &var_460;
                            return;
                        }
                        ebx = 2;
                        CloseHandle(rsi);
                        eax = ebx;
                        r11 = &var_460;
                        return;
                    }
                }
            }
        }
        CloseHandle(rsi);
    }
    eax = ebx;
    r11 = &var_460;
    return;
}
