// L1 structure recovery (3 blocks)
void str_probe_l1(void) {
    if (rcx == 0) {
        eax = 0xADDR;
        return;
    }
    eax = (uint32)(*(u8*)(rcx));
    return;
}
