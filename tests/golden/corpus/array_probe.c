// L1 structure recovery (3 blocks)
void array_probe_l1(void) {
    if (rcx == 0) {
        eax = 0xADDR;
        return;
    }
    eax = *(u32*)((rcx + ((int64)(edx) * 4)));
    return;
}
