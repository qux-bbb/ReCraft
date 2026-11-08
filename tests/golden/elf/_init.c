// L1 structure recovery (3 blocks)
void _init_l1(void) {
    rax = *(u64*)(0xADDR);
    if (rax != 0) {
        (*rax)();
    }
    return;
}
