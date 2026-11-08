// L1 structure recovery (1 blocks)
void array_probe_l1(void) {
    rdx = (int64)(esi);
    rax = rdx;
    eax += *(u32*)((rdi + (rdx * 4)));
    return;
}
