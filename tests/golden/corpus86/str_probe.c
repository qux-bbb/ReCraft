// L1 structure recovery (3 blocks)
void str_probe_l1(void) {
    eax = var_4;
    if (eax == 0) {
        eax = -1;
        return;
    }
    eax = (uint32)(*(u8*)(eax));
    return;
}
