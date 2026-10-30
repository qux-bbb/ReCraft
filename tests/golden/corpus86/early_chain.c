// L1 structure recovery (3 blocks)
void early_chain_l1(void) {
    eax = var_4;
    ecx = var_8;
    if (eax <= 0 /* signed */) {
        eax += ecx;
        eax = ((ecx > 0 /* signed */) ? ecx : eax);
    }
    return;
}
