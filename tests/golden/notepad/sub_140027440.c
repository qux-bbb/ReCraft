// L1 structure recovery (4 blocks)
void sub_140027440_l1(void) {
    r11 = 0;
    r10 = &var_18;
    r10 -= rax;
    __asm("cmovb r10, r11");
    __asm("mov r11, gs:[0xADDR]");
    if (r10 < r11) {
        r10w &= 0xADDR;
        do {
            r11 -= 0xADDR;
            *(u8*)(r11) = 0;
        } while (r10 != r11);
    }
    return;
}
