// L1 structure recovery (5 blocks)
void sub_1400066f4_l1(void) {
    *(u32*)(rcx) = 0;
    rdi = (rcx + 8);
    rax = *(u64*)((rcx + 0xADDR));
    rbx = rcx;
    *(u64*)((rcx + 0xADDR)) = 0;
    var_30 = rax;
    AcquireSRWLockExclusive(rdi);
    sub_140006b2c(rbx);
    if (rdi != 0) {
        ReleaseSRWLockExclusive(rdi);
    }
    edx = 0;
    sub_1400093f0(&var_30, 0);
    rbx = var_30;
    if (rbx != 0) {
        SetThreadpoolTimer(rbx, 0, 0, 0);
        WaitForThreadpoolTimerCallbacks(rbx, 1);
        CloseThreadpoolTimer(rbx);
    }
    return;
}
