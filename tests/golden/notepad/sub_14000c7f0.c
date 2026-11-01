// L1 structure recovery (11 blocks)
void sub_14000c7f0_l1(void) {
    rbp = &var_100;
    var_40 = 0;
    rbx = rcx;
    edx = 0;
    sub_1400029dc(&var_60, 0, 0xADDR);
    rax = qword_ADDR;
    rcx = qword_ADDR;
    var_64 = rax;
    rax = off_14002b268;
    var_60 = 0xADDR;
    var_a0 = rax;
    var_6c = rcx;
    var_78 = 8;
    var_7c = 0xADDR;
    var_9c = 2;
    LoadStringW(qword_ADDR, 0xADDR, rbp, 0xADDR);
    if (eax == 0) {
        eax = 6;
    } else {
        var_48 = 0;
        var_30 = &var_50;
        var_28 = 0;
        rax = &var_48;
        var_20 = rax;
        var_50 = rbx;
        FormatMessageW(0xADDR, rbp, 0, 0, &var_48, 0, &var_50);
        if (eax == 0) {
            eax = 6;
            var_40 = eax;
        } else {
            rax = var_48;
            var_8c = rax;
            ord_345(&var_60, &var_40, 0, 0);
            if (eax >= 0 /* signed */) {
                eax = var_40;
            } else {
                MessageBoxW(qword_ADDR, var_48, qword_ADDR, 0xADDR);
                var_40 = eax;
                return;
            }
        }
        rcx = var_48;
        if (rcx != 0) {
            CoTaskMemFree(var_48);
            eax = var_40;
        }
    }
    return;
}
