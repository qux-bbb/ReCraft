// L1 structure recovery (6 blocks)
void sub_140004e74_l1(void) {
    rsi = (rcx + 0xADDR);
    rdi = rcx;
    if (*(u8*)((rcx + 0xADDR)) == 0) {
        if (*(u64*)(rsi) == 0) {
            GetLastError();
            ebx = eax;
            CreateThreadpoolTimer(sub_140003030, rdi, 0);
            sub_1400093f0(rsi, rax);
            SetLastError(ebx);
        }
        rcx = *(u64*)(rsi);
        if (rcx != 0) {
            rax = 0xADDR;
            var_30 = rax;
            SetThreadpoolTimer(*(u64*)(rsi), &var_30, 0, 0xADDR);
            *(u8*)((rdi + 0xADDR)) = 1;
        }
    }
    return;
}
