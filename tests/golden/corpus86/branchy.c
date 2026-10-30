// L1 structure recovery (3 blocks)
void branchy_l1(void) {
    edx = var_4;
    if (edx <= 0xADDR /* signed */) {
        ecx = edx;
        eax = 0;
        ecx = -(ecx);
        eax = ((edx < -0xADDR /* signed */) ? ecx : eax);
        return;
    }
    eax = (edx + edx);
    return;
}
