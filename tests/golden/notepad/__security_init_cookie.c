// L1 structure recovery (3 blocks)
void __security_init_cookie_l1(void) {
    rax = qword_ADDR;
    rbx = 0xADDR;
    if (rax == 0xADDR) {
        var_48 = 0;
        var_40 = 0;
        GetSystemTimeAsFileTime(&var_48);
        rax = var_48;
        var_10 = rax;
        GetCurrentThreadId();
        var_10 ^= rax;
        GetCurrentProcessId();
        var_10 ^= rax;
        QueryPerformanceCounter(&var_40);
        eax = var_40;
        rcx = &var_10;
        rax <<= 0xADDR;
        rax ^= var_40;
        rax ^= var_10;
        rax ^= rcx;
        rax &= 0xADDR;
        rcx = 0xADDR;
        rax = ((rax == rbx) ? rcx : rax);
        qword_ADDR = rax;
    }
    rax = ~(rax);
    qword_ADDR = rax;
    return;
}
