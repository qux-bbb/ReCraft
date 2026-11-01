// L1 structure recovery (1 blocks)
void sub_140010070_l1(void) {
    *(u64*)(rdx) = off_1400291b8;
    *(__m128*)((rdx + 8)) = *(__m128*)((rcx + 8));
    return;
}
