// L1 structure recovery (7 blocks)
void sub_140010b30_l1(void) {
    rbx = rcx;
    GetLastError();
    if (eax != 5) {
        var_30 = 0;
        rcx = &var_40;
        var_28 = 0xADDR;
        var_20 = rcx;
        FormatMessageW(0xADDR, 0, eax, 0, &var_40, 0xADDR, 0);
        if (eax == 0) {
            r8 = qword_ADDR;
            var_20 = 0xADDR;
            sub_14000c3a4(qword_ADDR, qword_ADDR, r8, rbx, 0xADDR);
        } else {
            MessageBoxW(qword_ADDR, &var_40, qword_ADDR, 0xADDR);
        }
    } else {
        r8 = qword_ADDR;
        var_20 = 0xADDR;
        sub_14000c3a4(qword_ADDR, qword_ADDR, r8, rbx, 0xADDR);
        return;
    }
    return;
}
