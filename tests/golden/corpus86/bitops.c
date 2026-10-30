// L1 structure recovery (1 blocks)
void bitops_l1(void) {
    ecx = var_4;
    eax = ecx;
    eax >>= 2;
    ecx <<= 3;
    eax ^= ecx;
    eax |= 0xADDR;
    return;
}
