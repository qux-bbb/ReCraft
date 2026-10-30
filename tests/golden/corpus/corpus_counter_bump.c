// L1 structure recovery (1 blocks)
void corpus_counter_bump_l1(void) {
    eax = dword_ADDR;
    eax += 1;
    dword_ADDR = eax;
    return;
}
