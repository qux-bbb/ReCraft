// L1 structure recovery (3 blocks)
void branchy_l1(void) {
    edx = ecx;
    if ((ecx <= 0xADDR /* signed */)) {
        ecx = -(ecx);
        eax = (eax ^ eax);
        __asm("cmovl eax, ecx");
        return;
    }
    eax = (rcx + (rcx * 1));
    return;
}
