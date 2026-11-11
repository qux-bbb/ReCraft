// L1 structure recovery (3 blocks)
void tailish_l1(void) {
    eax = var_4;
    if (eax > 0 /* signed */) {
        edx = ((eax + 0xADDR) & -0xADDR);
        edx = -(edx);
        eax = ((eax + edx) + 0xADDR);
    }
    return;
}
