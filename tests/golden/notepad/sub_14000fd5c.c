// L1 structure recovery (7 blocks)
void sub_14000fd5c_l1(void) {
    var_3c = 0;
    var_40 = 0;
    SendMessageW(qword_ADDR, 0xADDR, &var_3c, &var_40, 0);
    eax = var_3c;
    if (eax == 0) {
        ecx = 1;
        var_40 = 1;
        var_3c = 1;
    } else {
        ecx = var_40;
    }
    r8 = qword_ADDR;
    edx = 0;
    eax *= 0xADDR;
    xmm0 = 0;
    var_38 = xmm0;
    var_28 = xmm0;
    __asm("div ecx");
    ebx = eax;
    eax = 0;
    var_18 = 0;
    sub_1400083f8(&var_38, 0xADDR, qword_ADDR, ebx);
    if (eax >= 0 /* signed */ && qword_ADDR != 0) {
        SendMessageW(qword_ADDR, 0xADDR, 2, &var_38);
    }
    dword_ADDR = ebx;
    return;
}
