// L1 structure recovery (1 blocks)
void recraft::clamped_l1(void) {
    eax = 0;
    eax = ((edi >= 0 /* signed */) ? edi : eax);
    return;
}
