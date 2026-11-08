// L1 structure recovery (1 blocks)
void field_probe_l1(void) {
    eax = *(u32*)((rdi + 4));
    eax += *(u32*)(rdi);
    eax += *(u32*)((rdi + 8));
    return;
}
