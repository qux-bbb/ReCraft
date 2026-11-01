// L1 structure recovery (7 blocks)
void sub_1400099b4_l1(void) {
    r10 = qword_ADDR;
    rbx = r9;
    rdi = rcx;
    if (r10 != 0) {
        rax = var_78;
        r9 = rbx;
        var_28 = rax;
        r8d = 0;
        rax = var_70;
        edx = 0;
        var_20 = rax;
        rcx = rdi;
        rax = r10;
        sub_140028010();
    } else {
        rax = qword_ADDR;
        if (rax == 0) {
            GetModuleHandleW("ntdll.dll");
            qword_ADDR = rax;
        }
        GetProcAddress(rax, "NtQueryWnfStateData");
        qword_ADDR = rax;
        r10 = rax;
        if (rax != 0) {
            rax = var_78;
            r9 = rbx;
            var_28 = rax;
            r8d = 0;
            rax = var_70;
            edx = 0;
            var_20 = rax;
            rcx = rdi;
            rax = r10;
            sub_140028010();
            return;
        }
        eax = 0xADDR;
    }
    return;
}
