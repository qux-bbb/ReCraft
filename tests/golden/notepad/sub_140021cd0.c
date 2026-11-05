// L1 structure recovery (13 blocks)
void sub_140021cd0_l1(void) {
    ebx = 0;
    r15d = ecx;
    rcx = qword_ADDR;
    var_68 = 0;
    var_70 = 0;
    var_78 = 0;
    if (rcx != 0) {
        wcsnlen(qword_ADDR, 0xADDR);
        r14 = rax;
    } else {
        r14d = ebx;
    }
    SendMessageW(qword_ADDR, 0xADDR, &var_68, &var_70);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    rdi = rax;
    if (rax == 0) {
        eax = 0;
    } else {
        LocalLock(rax);
        rsi = 0;
        if (rax == 0) {
            eax = 0;
            return;
        }
        eax = var_68;
        edx = var_70;
        edx -= var_68;
        rcx = (rsi + (rax * 2));
        rax = &var_78;
        var_20 = rax;
        sub_14001f8d8((0 + (rax * 2)), rdx, qword_ADDR);
        rdx = rax;
        eax = var_68;
        rcx = (rsi + (rax * 2));
        if (rdx == rcx) {
            eax = var_70;
            eax -= var_68;
            if (var_78 == eax) {
                SendMessageW(qword_ADDR, 0xADDR, 1, qword_ADDR);
                ecx = var_68;
                r8d = ecx;
                r9d = (r14 + rcx);
                SendMessageW(qword_ADDR, 0xADDR, ecx, (r14 + rcx));
                if (r15d != 0) {
                    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
                }
                ebx = 1;
            }
        }
        LocalUnlock(rdi);
        eax = ebx;
    }
    return;
}
