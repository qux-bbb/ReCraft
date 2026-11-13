// L1 structure recovery (3 blocks)
void __security_init_cookie_l1(void) {
    rax = qword_ADDR;
    rbx = 0xADDR;
    if (rax == 0xADDR) {
        var_40 = 0;
        GetSystemTimeAsFileTime(&var_40);
        rax = var_40;
        var_20 = rax;
        GetCurrentThreadId();
        var_20 ^= rax;
        GetCurrentProcessId();
        var_20 ^= rax;
        QueryPerformanceCounter(&var_48);
        eax = var_48;
        rcx = &var_20;
        rax <<= 0xADDR;
        rax ^= var_48;
        rax ^= var_20;
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
