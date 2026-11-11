// L1 structure recovery (1 blocks)
void early_chain_l1(void) {
    ecx = var_8;
    edx = var_c;
    ebx = ecx;
    eax = edx;
    ebx -= edx;
    eax -= ecx;
    eax = ((ecx > edx /* signed */) ? ebx : eax);
    return;
}
