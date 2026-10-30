// L1 structure recovery (1 blocks)
void float_math_l1(void) {
    xmm0 *= flt_ADDR;
    xmm0 += flt_ADDR;
    eax = (int32)(xmm0);
    return;
}
