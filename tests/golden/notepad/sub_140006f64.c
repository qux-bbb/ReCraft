// L1 structure recovery (7 blocks)
void sub_140006f64_l1(void) {
    rbx = rcx;
    sub_1400039e4(&var_20);
    AcquireSRWLockExclusive(rbx);
    if (*(u8*)((rbx + 0xADDR)) != 0) {
        sub_1400087e0(&var_20, (rbx + 8));
    }
    if (*(u8*)((rbx + 0xADDR)) != 0) {
        sub_1400087e0(&var_60, (rbx + 0xADDR));
    }
    if (*(u8*)((rbx + 0xADDR)) != 0) {
        sub_1400087e0(&var_a0, (rbx + 0xADDR));
    }
    ReleaseSRWLockExclusive(rbx);
    sub_140006a1c(&var_20);
    sub_1400040c8(&var_20);
    return;
}
