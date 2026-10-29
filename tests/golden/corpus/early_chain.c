// L1 structure recovery (3 blocks)
void early_chain_l1(void) {
    if (ecx <= 0 /* signed */) {
        eax = (rdx + rcx);
        eax = ((edx > 0 /* signed */) ? edx : eax);
        return;
    }
    eax = ecx;
    return;
}
