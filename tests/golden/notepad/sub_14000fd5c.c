// L1 structure recovery (7 blocks)
void sub_14000fd5c_l1(void) {
    var_24 = 0;
    var_20 = 0;
    SendMessageW(qword_ADDR, 0xADDR, &var_24, &var_20, 0);
    eax = var_24;
    if (eax == 0) {
        eax = 1;
        ecx = eax;
        var_20 = eax;
        var_24 = eax;
    } else {
        ecx = var_20;
    }
    r8 = qword_ADDR;
    edx = 0;
    eax *= 0xADDR;
    xmm0 = 0;
    var_28 = xmm0;
    var_38 = xmm0;
    __asm("div ecx");
    ebx = eax;
    var_48 = rax;
    sub_1400083f8(&var_28, (rax + 0xADDR), qword_ADDR, ebx);
    if (0 >= 0 /* signed */ && qword_ADDR != 0) {
        SendMessageW(qword_ADDR, 0xADDR, 2, &var_28);
    }
    dword_ADDR = ebx;
    return;
}
