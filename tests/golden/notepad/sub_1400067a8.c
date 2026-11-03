// L1 structure recovery (12 blocks)
void sub_1400067a8_l1(void) {
    r15d = 0;
    ebx = r9d;
    ebp = (uint32)(r8w);
    r14d = edx;
    rdi = rcx;
    if (*(u8*)(rcx) != 0xADDR && byte_ADDR == 0xADDR) {
        rax = qword_ADDR;
        if (rax == 0) {
            rsi = (rdi + 0xADDR);
            AcquireSRWLockExclusive(rsi);
            var_26 = r15w;
            var_20 = r14d;
            var_24 = bp;
            var_28 = ebx;
            sub_14000925c((rdi + 0xADDR), &var_20, 0xADDR);
            r14 = (rdi + 0xADDR);
            if (*(u8*)((rdi + 0xADDR)) == r15b) {
                if (*(u64*)(r14) == r15) {
                    GetLastError();
                    ebx = eax;
                    CreateThreadpoolTimer(sub_140002fb0, rdi, 0);
                    sub_1400093f0(r14, rax);
                    SetLastError(ebx);
                }
                rcx = *(u64*)(r14);
                if (rcx != 0) {
                    var_50 = 0xADDR;
                    SetThreadpoolTimer(*(u64*)(r14), &var_50, 0, 0xADDR);
                    *(u8*)((rdi + 0xADDR)) = 1;
                }
            }
            if (rsi != 0) {
                ReleaseSRWLockExclusive(rsi);
            }
        } else {
            sub_140028010();
        }
    }
    return;
}
