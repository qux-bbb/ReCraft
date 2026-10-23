// L1 structure recovery (1 blocks)
void float_math_l1(void) {
    __asm("mulss xmm0, [0xADDR]");
    __asm("addss xmm0, [0xADDR]");
    __asm("cvttss2si eax, xmm0");
    return;
}
