// L1 structure recovery (1 blocks)
void bitops_l1(void) {
    eax = ecx;
    ecx = (rcx * 8);
    eax = (eax >> 2);
    eax = (eax ^ ecx);
    eax = (eax | 0xADDR);
    return;
}
