// L1 structure recovery (3 blocks)
void array_probe_l1(void) {
    ecx = var_4;
    if (ecx == 0) {
        eax = -1;
        return;
    }
    eax = var_8;
    eax = *(u32*)((ecx + (eax * 4)));
    return;
}
