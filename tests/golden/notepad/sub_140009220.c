// L1 structure recovery (1 blocks)
void sub_140009220_l1(void) {
    *(u64*)(rdx) = "0B";
    *(__m128*)((rdx + 8)) = *(__m128*)((rcx + 8));
    *(f64*)((rdx + 0xADDR)) = *(f64*)((rcx + 0xADDR));
    return;
}
