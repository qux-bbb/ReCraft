// L1 structure recovery (4 blocks)
void sub_140001bc8_l1(void) {
    bl = cl;
    sub_1400025cc();
    edx = 0;
    if (eax != 0) {
        if (bl == 0) {
            __asm("xchg [0xADDR], rdx");
        }
    }
    return;
}
