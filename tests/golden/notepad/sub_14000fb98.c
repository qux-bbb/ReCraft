// L1 structure recovery (8 blocks)
void sub_14000fb98_l1(void) {
    bl = cl;
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    rdi = rax;
    SendMessageW(qword_ADDR, 0xADDR, eax, 0);
    esi = (rax + 1);
    SendMessageW(qword_ADDR, 0xADDR, eax, 0);
    edi -= eax;
    edi += 1;
    if (bl != 0) {
        edx = 0;
        sub_1400029dc(&var_32, 0, (rdx + 0xADDR));
        eax = 0xADDR;
        var_20 = edi;
        var_30 = ax;
        sub_1400083f8(&var_32, (rax + 0xADDR), qword_ADDR, esi, edi);
        if (eax >= 0 /* signed */ && qword_ADDR != 0) {
            SendMessageW(qword_ADDR, 0xADDR, 1, &var_30);
        }
        dword_ADDR = edi;
        dword_ADDR = esi;
    }
    r11 = &var_c0;
    return;
}
