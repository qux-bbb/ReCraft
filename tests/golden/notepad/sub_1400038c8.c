// L1 structure recovery (3 blocks)
void sub_1400038c8_l1(void) {
    rbp = var_15a0;
    r12 = r8;
    rsi = var_15a8;
    r15d = edx;
    rdi = var_15b0;
    r14 = rcx;
    rbx = var_15b8;
    edx = 0;
    r13 = r9;
    sub_1400029dc(&var_80, 0, 0xADDR);
    var_70 = &var_80;
    var_68 = 0xADDR;
    var_60 = &var_120;
    var_58 = 0xADDR;
    var_50 = &var_520;
    var_48 = 0;
    var_40 = rbx;
    var_38 = rdi;
    var_30 = 2;
    var_28 = rsi;
    var_20 = rbp;
    sub_140005f6c(r14, r15d, r12, r13, var_15a0, var_15a8, 2, var_15b0, var_15b8, 0, &var_520, 0xADDR, &var_120);
    if ((var_84 & 1) != 0) {
        sub_140008fec(&var_80);
    }
    return;
}
