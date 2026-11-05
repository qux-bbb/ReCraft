// L1 structure recovery (20 blocks)
void sub_14000d078_l1(void) {
    rbp = &var_100;
    rsi = rcx;
    edi = 0xADDR;
    edx = 0;
    sub_140009e44(&var_50, 0, edi);
    rbx = var_50;
    r15d = 0;
    if (rbx == 0) {
        edi = 0xADDR;
    } else {
        r14d = (rdi - 0xADDR);
        edx = 0;
        r8d = r14d;
        sub_1400029dc(&var_60);
        rax = qword_ADDR;
        rdx = "*.txt";
        var_68 = rax;
        rcx = rbx;
        rax = qword_ADDR;
        rdx -= rbx;
        var_70 = rax;
        var_60 = r14d;
        var_98 = edi;
        var_90 = rbx;
        do {
            rax = (rdi + 0xADDR);
            if (rax == 0) break;
            eax = (uint32)(*(u16*)((rdx + rcx)));
            if (ax == 0) break;
            *(u16*)(rcx) = ax;
            rcx += 2;
            rdi -= 1;
        } while (rdi != 0);
        var_40 = 0;
        rax = (rcx - 2);
        var_38 = 0;
        rax = ((rdi != 0) ? rcx : rax);
        *(u16*)(rax) = 0xADDR;
        var_b8 = qword_ADDR;
        rax = qword_ADDR;
        var_30 = rax;
        var_28 = r15;
        var_20 = r15;
        sub_1400083f8(rbp, 0xADDR, "%s%c*.txt%c%s%c*.*%c", qword_ADDR, r15, r15, qword_ADDR, r15, r15);
        var_c0 = 0xADDR;
        rax = "t";
        var_8c = 1;
        var_c8 = rax;
        rcx = rbp;
        rcx = ((eax < 0 /* signed */) ? var_78 : rcx);
        rax = sub_14000f170;
        var_78 = rcx;
        var_e0 = 0xADDR;
        var_d8 = sub_14000f170;
        SetThreadDpiAwarenessContext(-0xADDR);
        r14 = sub_14000f170;
        GetOpenFileNameW(&var_60);
        if (eax == 0) {
            GetModuleHandleA("comdlg32.dll");
            if (rax != 0) {
                MessageBoxW(var_68, "Not implemented.\nUse command line to specify file name instead.", "FileDialog", 0);
            }
            CommDlgExtendedError();
            if (eax != 0) {
                sub_14000f91c();
                edi = 0xADDR;
            } else {
                edi = 0xADDR;
            }
        } else {
            edi = r15d;
        }
        SetThreadDpiAwarenessContext(r14);
        if (edi >= 0 /* signed */) {
            sub_140022f54(rcx);
            if (dword_ADDR == r15d) {
                rcx = rbx;
                sub_14000ca10(rbx);
                dword_ADDR = eax;
            }
            sub_14000a0c0(rsi, &var_50);
            rbx = var_50;
        }
        if (rbx != 0) {
            CoTaskMemFree(rbx);
        }
    }
    eax = edi;
    return;
}
