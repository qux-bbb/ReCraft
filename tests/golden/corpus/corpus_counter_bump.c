// L1 structure recovery (1 blocks)
void corpus_counter_bump_l1(void) {
    eax = *(u32*)(0xADDR);
    eax = (eax + 1);
    *(u32*)(0xADDR) = eax;
    return;
}
