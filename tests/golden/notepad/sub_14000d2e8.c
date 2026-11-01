// L1 structure recovery (18 blocks)
void sub_14000d2e8_l1(void) {
    rbp = &var_100;
    r14 = rdx;
    rsi = rcx;
    edi = 0xADDR;
    edx = 0;
    sub_140009e44(&var_50, 0, edi);
    rbx = var_50;
    r12d = 0;
    if (rbx == 0) {
        edi = 0xADDR;
    } else {
        edx = 0;
        sub_1400029dc(&var_60, 0, (rdi - 0xADDR));
        rax = qword_ADDR;
        rcx = rbx;
        var_68 = rax;
        rsi -= rbx;
        rax = qword_ADDR;
        var_70 = rax;
        var_60 = 0xADDR;
        var_98 = edi;
        var_90 = rbx;
        do {
            rax = (rdi + 0xADDR);
            if (rax == 0) break;
            eax = (uint32)(*(u16*)((rsi + rcx)));
            if (ax == 0) break;
            *(u16*)(rcx) = ax;
            rcx += 2;
            rdi -= 1;
        } while (rdi != 0);
        var_40 = r12;
        rax = (rcx - 2);
        var_38 = r12;
        rax = ((rdi != 0) ? rcx : rax);
        *(u16*)(rax) = r12w;
        var_b8 = qword_ADDR;
        var_d8 = sub_14000f1d0;
        rax = qword_ADDR;
        var_30 = rax;
        var_28 = r12;
        var_20 = r12;
        var_c0 = 0xADDR;
        var_e0 = 0xADDR;
        sub_1400083f8(rbp, 0xADDR, "%s%c*.txt%c%s%c*.*%c", qword_ADDR, r12, r12, qword_ADDR, r12, r12);
        var_8c = 1;
        rax = "t";
        rcx = rbp;
        var_c8 = rax;
        rcx = ((eax < 0 /* signed */) ? var_78 : rcx);
        var_78 = rcx;
        SetThreadDpiAwarenessContext(-0xADDR);
        dword_ADDR = 1;
        rsi = rax;
        GetSaveFileNameW(&var_60);
        if (eax == 0) {
            GetModuleHandleA("comdlg32.dll");
            if (rax != 0) {
                MessageBoxW(var_68, "Not implemented.", "FileDialog", 0);
            }
            CommDlgExtendedError();
            if (eax != 0) {
                sub_14000f91c();
                edi = 0xADDR;
            } else {
                edi = 0xADDR;
            }
        } else {
            edi = r12d;
        }
        dword_ADDR = r12d;
        SetThreadDpiAwarenessContext(rsi);
        if (edi >= 0 /* signed */) {
            sub_14000a0c0(r14, &var_50);
            rbx = var_50;
        }
        if (rbx != 0) {
            CoTaskMemFree(rbx);
        }
    }
    eax = edi;
    r11 = &var_1b0;
    return;
}
