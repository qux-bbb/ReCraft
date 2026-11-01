// L1 structure recovery (5 blocks)
void sub_1400057d8_l1(void) {
    GetLastError();
    if (eax != 0) {
        if (eax > 0 /* signed */) {
            eax = (uint32)(ax);
            eax |= 0xADDR;
        }
    } else {
        rax = var_48;
        r9d = 0;
        var_30 = 0xADDR;
        r8d = 0;
        var_28 = rax;
        edx = 0;
        var_20 = 0;
        ecx = 0;
        sub_1400032f0(0, 0, 0, 0, 0, var_48, 0xADDR);
        eax = (uint32)(ax);
        eax |= 0xADDR;
        return;
    }
    return;
}
