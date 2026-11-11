// L1 structure recovery (4 blocks)
void count_bits_l1(void) {
    eax = var_4;
    ecx = 0;
    if (eax != 0) {
        do {
            edx = (eax + 0xADDR);
            ecx += 1;
            eax &= edx;
        } while (eax != 0);
    }
    eax = ecx;
    return;
}
