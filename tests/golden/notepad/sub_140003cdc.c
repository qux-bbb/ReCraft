// L1 structure recovery (9 blocks)
void sub_140003cdc_l1(void) {
    rbx = rcx;
    sub_1400039e4(&var_20);
    if (*(u8*)((rbx + 0xADDR)) != 0) {
        sub_1400087e0(&var_20, (rbx + 8));
    }
    if (*(u8*)((rbx + 0xADDR)) != 0) {
        sub_1400087e0(&var_60, (rbx + 0xADDR));
    }
    if (*(u8*)((rbx + 0xADDR)) != 0) {
        sub_1400087e0(&var_a0, (rbx + 0xADDR));
    }
    sub_140006a1c(&var_20);
    sub_1400040c8(&var_20);
    rsi = *(u64*)((rbx + 0xADDR));
    *(u64*)((rbx + 0xADDR)) = 0;
    if (rsi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rsi);
    }
    DeleteCriticalSection((rbx + 0xADDR));
    rcx = (rbx + 8);
    r11 = &var_e0;
    rbx = *(u64*)((r11 + 0xADDR));
    rsi = *(u64*)((r11 + 0xADDR));
    return sub_1400040c8(rcx);
}
