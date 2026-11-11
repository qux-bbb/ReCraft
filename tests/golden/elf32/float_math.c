// L1 structure recovery (1 blocks)
void float_math_l1(void) {
    __asm("fnstcw [esp+0xADDR]");
    __asm("fld st, dword ptr [esp+0xADDR]");
    eax = (uint32)(var_6);
    __asm("fadd st, st(0)");
    ah |= 0xADDR;
    var_4 = ax;
    __asm("fldcw [esp+0xADDR]");
    __asm("fistp dword ptr [esp], st");
    __asm("fldcw [esp+0xADDR]");
    eax = (var_0 + 1);
    return;
}
