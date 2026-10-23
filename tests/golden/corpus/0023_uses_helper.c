// L1 structure recovery (1 blocks)
void uses_helper_l1(void) {
    eax = (rcx + 1);
    var_10 = ecx;
    var_8 = eax;
    ecx = var_8;
    eax = var_10;
    __asm("imul eax, ecx");
    eax = (eax + 7);
    return;
}
