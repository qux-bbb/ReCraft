// L1 structure recovery (7 blocks)
void sub_140009a5c_l1(void) {
    r10 = qword_ADDR;
    ebx = r8d;
    rdi = rdx;
    rsi = rcx;
    if (r10 != 0) {
        eax = var_80;
        r9d = 0;
        var_30 = eax;
        r8d = ebx;
        eax = var_78;
        rdx = rdi;
        var_28 = eax;
        rcx = rsi;
        var_20 = 0;
        rax = r10;
        sub_140028010();
    } else {
        rax = qword_ADDR;
        if (rax == 0) {
            GetModuleHandleW("ntdll.dll");
            qword_ADDR = rax;
        }
        GetProcAddress(rax, "NtUpdateWnfStateData");
        qword_ADDR = rax;
        r10 = rax;
        if (rax != 0) {
            eax = var_80;
            r9d = 0;
            var_30 = eax;
            r8d = ebx;
            eax = var_78;
            rdx = rdi;
            var_28 = eax;
            rcx = rsi;
            var_20 = 0;
            rax = r10;
            sub_140028010();
            return;
        }
        eax = 0xADDR;
    }
    return;
}
