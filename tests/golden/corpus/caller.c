// L1 structure recovery (3 blocks)
void caller_l1(void) {
    if (ecx <= 0xADDR /* signed */) {
        eax = 0;
        ecx = ((ecx < 0xADDR /* signed */) ? eax : ecx);
        eax = ecx;
        return;
    }
    eax = (rcx + (rcx * 2));
    return;
}
