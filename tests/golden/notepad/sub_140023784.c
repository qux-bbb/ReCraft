// L1 structure recovery (5 blocks)
void sub_140023784_l1(void) {
    if (dword_ADDR > 4) {
        rdx = 0xADDR;
        if ((qword_ADDR & 0xADDR) != 0) {
            rax = qword_ADDR;
            rax &= rdx;
            if (rax == qword_ADDR) {
                rax = qword_ADDR;
                rdx = off_14002ed0b;
                rax += 1;
                qword_ADDR = rax;
                *(u64*)((r11 + 8)) = rax;
                *(u64*)((r11 + 0xADDR)) = off_140036178;
                *(u64*)((r11 - 0xADDR)) = (r11 + 8);
                rax = (r11 + 0xADDR);
                *(u64*)((r11 - 0xADDR)) = rax;
                sub_140001008(rcx);
            }
        }
    }
    return;
}
