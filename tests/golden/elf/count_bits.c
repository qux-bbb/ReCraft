// L1 structure recovery (4 blocks)
void count_bits_l1(void) {
    edx = 0;
    if (edi != 0) {
        do {
            eax = (rdi - 1);
            edx += 1;
            edi &= eax;
        } while (edi != 0);
    }
    eax = edx;
    return;
}
