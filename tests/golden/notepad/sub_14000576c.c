// L1 structure recovery (3 blocks)
void sub_14000576c_l1(void) {
    rbx = r8;
    edi = edx;
    rsi = rcx;
    GetLastError();
    if (eax == 0) {
        rax = var_78;
        r9d = 0;
        var_30 = 0xADDR;
        var_28 = rax;
        var_20 = 0;
        sub_1400032f0(rsi, edi, rbx, 0, 0, var_78, 0xADDR);
        eax = 0xADDR;
    }
    return;
}
