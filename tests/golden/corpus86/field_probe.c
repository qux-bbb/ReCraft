// L1 structure recovery (3 blocks)
void field_probe_l1(void) {
    eax = var_4;
    if (eax == 0) {
        eax = -1;
        return;
    }
    eax = *(u32*)((eax + 0xADDR));
    return;
}
