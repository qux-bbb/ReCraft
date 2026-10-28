// L1 structure recovery (1 blocks)
void bitops_l1(void) {
    eax = ecx;
    ecx = (rcx * 8);
    eax >>= 2;
    eax ^= ecx;
    eax |= 0xADDR;
    return;
}
