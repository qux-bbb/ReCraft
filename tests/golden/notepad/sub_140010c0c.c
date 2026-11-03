// L1 structure recovery (12 blocks)
void sub_140010c0c_l1(void) {
    r13 = qword_ADDR;
    ebx = 0;
    r15 = r9;
    r12d = r8d;
    edi = edx;
    var_80 = ebx;
    var_40 = ebx;
    if (var_a0 != 0) {
        rsi = rbx;
        if (edi != 0xADDR) {
            rsi = &var_80;
            rsi = ((edi == 0xADDR) ? rbx : rsi);
        }
        var_38 = rsi;
        var_30 = rbx;
        var_28 = ebx;
        var_20 = rbx;
        WideCharToMultiByte(edi, r12d, r15, var_a0, rbx, ebx, rbx, rsi);
        ebp = eax;
        if (eax != 0) {
            LocalAlloc(0xADDR, (rax + 1));
            rbx = rax;
            if (rax != 0) {
                var_38 = rsi;
                var_30 = 0;
                var_28 = ebp;
                var_20 = rbx;
                WideCharToMultiByte(edi, r12d, r15, var_a0, rbx, ebp, 0, rsi);
                edi = eax;
                if (eax != 0) {
                    var_20 = 0;
                    WriteFile(r13, rbx, ebp, &var_40, 0);
                    edi = eax;
                }
                LocalFree(rbx);
                eax = edi;
            } else {
                SetLastError((rax + 8));
                eax = 0;
            }
        } else {
            eax = 0;
            r11 = &var_50;
            return;
        }
    } else {
        eax = (rbx + 1);
    }
    r11 = &var_50;
    return;
}
