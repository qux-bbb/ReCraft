// L1 structure recovery (5 blocks)
void sub_140022f54_l1(void) {
    var_50 = ecx;
    edx = dword_ADDR;
    if (dword_ADDR > 4) {
        r8 = 0xADDR;
        if ((qword_ADDR & 0xADDR) != 0) {
            rax = qword_ADDR;
            rax &= r8;
            if (rax == qword_ADDR) {
                rax = (qword_ADDR + 1);
                var_50 = edx;
                qword_ADDR = rax;
                rdx = off_14002e801;
                *(u64*)((r11 + 0xADDR)) = rax;
                *(u64*)((r11 + 0xADDR)) = off_140036178;
                *(u64*)((r11 - 0xADDR)) = (r11 + 8);
                *(u64*)((r11 - 0xADDR)) = (r11 + 0xADDR);
                rax = (r11 + 0xADDR);
                *(u64*)((r11 - 0xADDR)) = rax;
                sub_1400011bc(rcx);
            }
        }
    }
    return;
}
