// L1 structure recovery (5 blocks)
void sub_14000bf4c_l1(void) {
    rsi = rdx;
    rdi = rcx;
    if (rcx != rdx) {
        rbp = *(u64*)(rcx);
        r14 = *(u64*)(rdx);
        if (rbp != 0) {
            GetLastError();
            ebx = eax;
            LocalFree(rbp);
            SetLastError(ebx);
        }
        *(u64*)(rdi) = r14;
        *(u64*)(rsi) = 0;
    }
    rax = rdi;
    return;
}
