// L1 structure recovery (1 blocks)
void array_probe_l1(void) {
    eax = var_8;
    eax += *(u32*)((var_4 + (eax * 4)));
    return;
}
