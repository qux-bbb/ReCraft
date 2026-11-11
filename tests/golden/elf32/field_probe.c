// L1 structure recovery (1 blocks)
void field_probe_l1(void) {
    edx = var_4;
    eax = *(u32*)((edx + 4));
    eax += *(u32*)(edx);
    eax += *(u32*)((edx + 8));
    return;
}
