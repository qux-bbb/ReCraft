// L1 structure recovery (3 blocks)
void array_probe_l1(void) {
    if (((rcx & rcx) == 0)) {
        eax = 0xADDR;
        return;
    }
    rax = (int64*)(edx);
    eax = *(u32*)((rcx + (rax * 4)));
    return;
}
