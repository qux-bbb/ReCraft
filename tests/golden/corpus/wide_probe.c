// L1 structure recovery (3 blocks)
void wide_probe_l1(void) {
    if (((rcx & rcx) == 0)) {
        eax = 0xADDR;
        return;
    }
    eax = (uint32*)(*(u16*)(rcx));
    return;
}
