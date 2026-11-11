// L1 structure recovery (1 blocks)
void corpus_counter_bump_l1(void) {
    __x86.get_pc_thunk.dx();
    edx += 0xADDR;
    eax = (*(u32*)((edx + 0xADDR)) + 1);
    *(u32*)((edx + 0xADDR)) = eax;
    return;
}
