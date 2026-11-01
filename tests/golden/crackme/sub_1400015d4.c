// L1 structure recovery (4 blocks)
void sub_1400015d4_l1(void) {
    bl = cl;
    sub_140001ccc();
    edx = 0;
    if (eax != 0 && bl == 0) {
        __asm("xchg [0xADDR], rdx");
    }
    return;
}
