// L1 structure recovery (14 blocks)
void sub_14000fe3c_l1(void) {
    var_48 = 0;
    rbx = qword_ADDR;
    if (rbx != 0) {
        edx = 0;
        sub_1400029dc(&var_e0, 0, 0xADDR);
        FindFirstFileW(rbx, &var_e0);
        rbx = rax;
        var_70 = off_1400291e0;
        rcx = &var_48;
        rdx = &var_68;
        if (rax == -1) {
            var_40 = qword_ADDR;
            var_78 = &var_40;
            rax = &var_70;
            var_d8 = rax;
            sub_14000b91c(&var_48, &var_68);
        } else {
            var_40 = &var_10c;
            var_78 = &var_40;
            rax = &var_70;
            var_d8 = rax;
            sub_14000b91c(&var_48, &var_68);
            FindClose(rbx);
        }
    } else {
        sub_14000be08(&var_40, qword_ADDR, -1);
        sub_14000bf4c(&var_48, &var_40);
        rcx = var_40;
        if (rcx != 0) {
            LocalFree(var_40);
        }
    }
    rbx = var_48;
    if (rbx != 0) {
        sub_14000d990();
        rcx = "*";
        dil = al;
        var_60 = rbx;
        rax = 0xADDR;
        rcx = ((dil == 0) ? rax : rcx);
        var_50 = 0;
        rax = &var_58;
        var_58 = rcx;
        var_30 = rax;
        var_28 = 0;
        rax = &var_50;
        var_20 = rax;
        FormatMessageW(0xADDR, qword_ADDR, 0, 0, &var_50, 0, &var_58);
        rcx = qword_ADDR;
        rdx = var_50;
        if (eax == 0) {
            rdx = rbx;
        }
        SetWindowTextW(qword_ADDR);
        rcx = var_50;
        byte_ADDR = dil;
        if (rcx != 0) {
            LocalFree(var_50);
        }
        if (rbx != 0) {
            LocalFree(rbx);
        }
    }
    r11 = &var_340;
    return;
}
