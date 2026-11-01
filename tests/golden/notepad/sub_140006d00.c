// L1 structure recovery (1 blocks)
void sub_140006d00_l1(void) {
    dword_ADDR = ecx;
    eax = 1;
    __asm("lock xadd [0xADDR], eax");
    eax += 1;
    return;
}
