// L1 structure recovery (1 blocks)
void corpus_counter_bump_l1(void) {
    *(u32*)(0xADDR) += 1;
    return;
}
