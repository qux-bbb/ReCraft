// L1 structure recovery (3 blocks)
void caller_l1(void) {
    eax = var_4;
    if (eax <= 0xADDR /* signed */) {
        eax = ((eax < -0xADDR /* signed */) ? 0 : eax);
        return;
    }
    eax += (eax * 2);
    return;
}
