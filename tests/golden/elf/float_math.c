// L1 structure recovery (1 blocks)
void float_math_l1(void) {
    xmm0 += xmm0;
    eax = (int32)(xmm0);
    eax += 1;
    return;
}
