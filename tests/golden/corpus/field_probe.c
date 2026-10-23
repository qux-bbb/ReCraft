// L1 structure recovery (3 blocks)
void field_probe_l1(void) {
    if (((rcx & rcx) == 0)) {
        eax = 0xADDR;
        return;
    }
    eax = *(u32*)((rcx + 0xADDR));
    return;
}
