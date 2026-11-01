// L1 structure recovery (3 blocks)
void sub_1400093f0_l1(void) {
    rsi = *(u64*)(rcx);
    rbp = rdx;
    rdi = rcx;
    if (rsi != 0) {
        GetLastError();
        ebx = eax;
        SetThreadpoolTimer(rsi, 0, 0, 0);
        WaitForThreadpoolTimerCallbacks(rsi, 1);
        CloseThreadpoolTimer(rsi);
        SetLastError(ebx);
    }
    *(u64*)(rdi) = rbp;
    return;
}
