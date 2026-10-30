// L1 structure recovery (1 blocks)
void float_math_l1(void) {
    xmm0 = var_4;
    xmm0 *= *(f32*)(0xADDR);
    xmm0 += *(f32*)(0xADDR);
    eax = (int32)(xmm0);
    return;
}
