// L1 structure recovery (3 blocks)
void sub_140008f80_l1(void) {
    rsi = rcx;
    ebx = r8d;
    rdi = rdx;
    GetModuleHandleW("kernelbase.dll");
    GetProcAddress(rax, "RaiseFailFastException");
    if (rax != 0) {
        r8d = ebx;
        rdx = rdi;
        rcx = rsi;
        sub_140028010(rsi, rdi, ebx);
    }
    return;
}
