// L1 structure recovery (5 blocks)
void sub_140023308_l1(void) {
    if (dword_ADDR > 4) {
        if ((qword_ADDR & 0xADDR) != 0) {
            rax = (qword_ADDR & 0xADDR);
            if (rax == qword_ADDR) {
                var_98 = *(u32*)((rcx + 0xADDR));
                var_a0 = *(u32*)((rcx + 4));
                eax = *(u32*)(rcx);
                var_a8 = rax;
                var_20 = *(u64*)((rcx + 8));
                var_18 = (rcx + 0xADDR);
                rax = (qword_ADDR + 1);
                qword_ADDR = rax;
                var_10 = rax;
                var_8 = off_140036178;
                *(u64*)((r11 - 0xADDR)) = &var_98;
                *(u64*)((r11 - 0xADDR)) = &var_a0;
                *(u64*)((r11 - 0xADDR)) = &var_a8;
                *(u64*)((r11 - 0xADDR)) = &var_20;
                *(u64*)((r11 - 0xADDR)) = &var_18;
                *(u64*)((r11 - 0xADDR)) = &var_10;
                rax = &var_8;
                *(u64*)((r11 - 0xADDR)) = rax;
                sub_140001094(rcx, off_14002ec0c);
            }
        }
    }
    return;
}
