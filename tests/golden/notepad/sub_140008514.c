// L1 structure recovery (13 blocks)
void sub_140008514_l1(void) {
    *(u64*)(rdx) = 0;
    rbp = r9;
    r15 = r8;
    r14 = rdx;
    rbx = rcx;
    if (*(u8*)(rcx) != 0) {
        rdi = (rcx + 0xADDR);
        AcquireSRWLockExclusive(rdi);
        rsi = (rbx + 0xADDR);
        if (*(u64*)(rsi) != 0) {
            sub_140008748((rbx + 0xADDR), r14, r15, rbp);
        } else {
            *(u64*)(rsi) = 0;
            rax = qword_ADDR;
            if (rax != 0) {
                r9 = rsi;
                r8d = 0;
                rdx = rbx;
                sub_140028010(sub_140002f80);
            } else {
                rax = qword_ADDR;
                if (rax == 0) {
                    GetModuleHandleW("ntdll.dll");
                    qword_ADDR = rax;
                }
                GetProcAddress(rax, "RtlRegisterFeatureConfigurationChangeNotification");
                qword_ADDR = rax;
                if (rax == 0) {
                    eax = 0xADDR;
                }
            }
        }
        if (rdi != 0) {
            ReleaseSRWLockExclusive(rdi);
        }
    }
    return;
}
