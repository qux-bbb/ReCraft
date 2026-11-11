// L1 structure recovery (3 blocks)
void _init_l1(void) {
    __x86.get_pc_thunk.bx();
    ebx += 0xADDR;
    eax = dword_ADDR[ebx];
    if (eax != 0) {
        (*eax)();
    }
    return;
}
