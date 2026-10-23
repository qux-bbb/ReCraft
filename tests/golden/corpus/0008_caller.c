// L1 structure recovery (3 blocks)
void caller_l1(void) {
    if ((ecx <= 0xADDR /* signed */)) {
        eax = (eax ^ eax);
        __asm("cmovl ecx, eax");
        eax = ecx;
        return;
    }
    eax = (rcx + (rcx * 2));
    return;
}
