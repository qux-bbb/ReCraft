// L1 structure recovery (3 blocks)
void count_bits_l1(void) {
    ecx = var_4;
    eax = 0;
    do {
        edx = ecx;
        edx &= 1;
        eax += edx;
        ecx >>= 1;
    } while (ecx != 0);
    return;
}
