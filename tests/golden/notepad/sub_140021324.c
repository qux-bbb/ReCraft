// L1 structure recovery (18 blocks)
void sub_140021324_l1(void) {
    ebx = ecx;
    SetCursor(qword_ADDR);
    if (ebx == 0) {
        sub_140020598();
    } else {
        ebx -= 1;
        if (ebx == 0) {
            sub_14002048c();
        } else if (ebx == 1) {
            sub_140020314();
        }
    }
    if (rax != 0) {
        sub_1400206b4(rax);
        r8d = eax;
        if (eax != 0) {
            eax += 3;
            if (eax > 1) {
                if (r8d != -0xADDR) {
                    if (r8d != -0xADDR) {
                        if (r8d != -1) {
                            var_30 = 0;
                            rax = &var_40;
                            var_28 = 0xADDR;
                            var_20 = rax;
                            FormatMessageW(0xADDR, 0, r8, 0);
                            r9 = qword_ADDR;
                            r8 = &var_40;
                            r8 = ((eax == 0) ? qword_ADDR : r8);
                            var_20 = 0xADDR;
                            r9 = ((dword_ADDR != 0) ? qword_ADDR : r9);
                            sub_14000c3a4(qword_ADDR, qword_ADDR, r8, r9, 0xADDR);
                        } else {
                            GetLastError();
                            r8d = eax;
                            if (eax != 0) {
                                var_30 = 0;
                                rax = &var_40;
                                var_28 = 0xADDR;
                                var_20 = rax;
                                FormatMessageW(0xADDR, 0, r8, 0);
                                r9 = qword_ADDR;
                                r8 = &var_40;
                                r8 = ((eax == 0) ? qword_ADDR : r8);
                                var_20 = 0xADDR;
                                r9 = ((dword_ADDR != 0) ? qword_ADDR : r9);
                                sub_14000c3a4(qword_ADDR, qword_ADDR, r8, r9, 0xADDR);
                                return;
                            }
                        }
                    } else {
                        r8d = 0xADDR;
                        var_30 = 0;
                        rax = &var_40;
                        var_28 = 0xADDR;
                        var_20 = rax;
                        FormatMessageW(0xADDR, 0, r8, 0);
                        r9 = qword_ADDR;
                        r8 = &var_40;
                        r8 = ((eax == 0) ? qword_ADDR : r8);
                        var_20 = 0xADDR;
                        r9 = ((dword_ADDR != 0) ? qword_ADDR : r9);
                        sub_14000c3a4(qword_ADDR, qword_ADDR, r8, r9, 0xADDR);
                        return;
                    }
                } else {
                    r8d = 0xADDR;
                    var_30 = 0;
                    rax = &var_40;
                    var_28 = 0xADDR;
                    var_20 = rax;
                    FormatMessageW(0xADDR, 0, r8, 0);
                    r9 = qword_ADDR;
                    r8 = &var_40;
                    r8 = ((eax == 0) ? qword_ADDR : r8);
                    var_20 = 0xADDR;
                    r9 = ((dword_ADDR != 0) ? qword_ADDR : r9);
                    sub_14000c3a4(qword_ADDR, qword_ADDR, r8, r9, 0xADDR);
                    return;
                }
            }
        }
    } else {
        SetCursor(qword_ADDR);
    }
    return;
}
