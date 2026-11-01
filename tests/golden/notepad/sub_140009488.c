// L1 structure recovery (5 blocks)
void sub_140009488_l1(void) {
    rdi = *(u64*)(rcx);
    rsi = rdx;
    rbx = rcx;
    if (rdi == 0) {
        *(u64*)(rbx) = rsi;
        return;
    }
    GetLastError();
    ebp = eax;
    CloseHandle(rdi);
    if (eax == 0) {
        sub_1400091e4(var_28, 0xADDR);
    }
    SetLastError(ebp);
}
