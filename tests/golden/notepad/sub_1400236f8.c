// L1 structure recovery (5 blocks)
void sub_1400236f8_l1(void) {
    if (dword_ADDR > 4) {
        r8 = 0xADDR;
        if ((qword_ADDR & 0xADDR) != 0) {
            rax = qword_ADDR;
            rax &= r8;
            if (rax == qword_ADDR) {
                rax = qword_ADDR;
                rdx = off_14002ecbe;
                rax += 1;
                var_58 = ecx;
                qword_ADDR = rax;
                *(u64*)((r11 + 0xADDR)) = rax;
                *(u64*)((r11 + 0xADDR)) = off_140036178;
                *(u64*)((r11 - 0xADDR)) = (r11 + 0xADDR);
                *(u64*)((r11 - 0xADDR)) = (r11 + 0xADDR);
                rax = (r11 + 0xADDR);
                *(u64*)((r11 - 0xADDR)) = rax;
                sub_1400011bc(rcx);
            }
        }
    }
    return;
}
