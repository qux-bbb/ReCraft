// L1 structure recovery (1 blocks)
void early_chain_l1(void) {
    edx = edi;
    eax = esi;
    edx -= esi;
    eax -= edi;
    eax = ((edi > esi /* signed */) ? edx : eax);
    return;
}
