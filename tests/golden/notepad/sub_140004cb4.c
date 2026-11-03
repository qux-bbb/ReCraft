// L1 structure recovery (14 blocks)
void sub_140004cb4_l1(void) {
    rbx = rcx;
    if (*(u64*)((rcx + 0xADDR)) == 0) {
        GetLastError();
        ebp = eax;
        if (*(u64*)((rbx + 0xADDR)) == 0) {
            if (*(u64*)((rbx + 0xADDR)) == 0) {
                var_30 = 0;
                sub_14000468c(*(u64*)((rbx + 8)), &var_30);
                if (eax >= 0 /* signed */ && *(u64*)((rbx + 0xADDR)) == 0) {
                    *(u64*)((rbx + 0xADDR)) = var_30;
                }
            }
            rax = *(u64*)((rbx + 0xADDR));
            rcx = (rax + 0xADDR);
            rax = -(rax);
            rdi = ((rax != 0) ? rcx : 0);
        } else {
        }
        rsi = (rbx + 0xADDR);
        AcquireSRWLockExclusive(rsi);
        if (*(u64*)((rbx + 0xADDR)) == 0) {
            *(u64*)((rbx + 0xADDR)) = 0;
        }
        if (rsi != 0) {
            ReleaseSRWLockExclusive(rsi);
        }
        SetLastError(ebp);
    }
    al = (uint8)((*(u64*)((rbx + 0xADDR)) != 0));
    return;
}
