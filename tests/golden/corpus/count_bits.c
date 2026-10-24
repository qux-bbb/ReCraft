// L1 structure recovery (3 blocks)
void count_bits_l1(void) {
    eax = 0;
    do {
        edx = ecx;
        edx = (edx & 1);
        eax = (eax + edx);
        ecx = (ecx >> 1);
    } while (((eax + edx) != 0));
    return;
}
