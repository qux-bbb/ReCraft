// L1 structure recovery (5 blocks)
void sub_14001e9b0_l1(void) {
    var_50 = 0;
    ebx = r8d;
    var_68 = 1;
    var_30 = 4;
    if (rcx != 0) {
        *(u64*)((r11 - 0xADDR)) = (r11 - 0xADDR);
        *(u64*)((r11 - 0xADDR)) = (r11 + 0xADDR);
        RegQueryValueExW(rcx, rdx, 0, (r11 + 8));
        if (eax == 0 && var_50 == 4) {
            ebx = var_68;
        }
    }
    eax = ebx;
    return;
}
