// L1 structure recovery (5 blocks)
void sub_1400031d8_l1(void) {
    rdi = r8;
    esi = edx;
    rbp = rcx;
    GetLastError();
    r14 = var_98;
    ebx = eax;
    if (eax != 0) {
        if (eax > 0 /* signed */) {
            ebx = (uint32)(bx);
            ebx |= 0xADDR;
        }
    } else {
        var_30 = 0xADDR;
        r9d = 0;
        var_28 = r14;
        var_20 = 0;
        sub_1400032f0(rbp, esi, rdi, 0, 0, r14, 0xADDR);
        ebx = (uint32)(bx);
        ebx |= 0xADDR;
        var_50 = ebx;
        sub_140005cc0(ebx);
        var_58 = 0;
        r8 = rdi;
        var_54 = eax;
        edx = esi;
        rax = &var_50;
        rcx = rbp;
        var_30 = rax;
        var_28 = r14;
        sub_1400030dc();
        r11 = &var_60;
        eax = ebx;
        return;
    }
    var_50 = ebx;
    sub_140005cc0(ebx);
    var_58 = 0;
    r8 = rdi;
    var_54 = eax;
    edx = esi;
    rax = &var_50;
    rcx = rbp;
    var_30 = rax;
    var_28 = r14;
    sub_1400030dc();
    r11 = &var_60;
    eax = ebx;
    return;
}
