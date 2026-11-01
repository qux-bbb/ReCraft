// L1 structure recovery (3 blocks)
void count_bits_l1(void) {
    eax = 0;
    do {
        edx = (ecx & 1);
        eax += edx;
        ecx >>= 1;
    } while (ecx != 0);
    return;
}
