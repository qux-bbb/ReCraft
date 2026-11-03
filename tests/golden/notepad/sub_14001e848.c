// L1 structure recovery (16 blocks)
void sub_14001e848_l1(void) {
    eax = *(u32*)(rcx);
    rbx = r8;
    rdi = rcx;
    if (eax != 0 && byte_ADDR == 0) {
        rax = qword_ADDR;
        if (rax == 0) {
            rsi = (rdi + 8);
            AcquireSRWLockExclusive(rsi);
            eax = *(u32*)(rdi);
            if (eax != 0 && byte_ADDR == 0) {
                rax = qword_ADDR;
                if (rax == 0) {
                    var_20 = 0xADDR;
                    var_28 = rbx;
                    sub_14000925c((rdi + 0xADDR), &var_20, 0xADDR);
                    r14 = (rdi + 0xADDR);
                    if (*(u8*)((rdi + 0xADDR)) == 0) {
                        if (*(u64*)(r14) == 0) {
                            GetLastError();
                            ebx = eax;
                            CreateThreadpoolTimer(sub_1400124e0, rdi, 0);
                            sub_1400093f0(r14, rax);
                            SetLastError(ebx);
                        }
                        rcx = *(u64*)(r14);
                        if (rcx != 0) {
                            var_40 = 0xADDR;
                            SetThreadpoolTimer(*(u64*)(r14), &var_40, 0, 0xADDR);
                            *(u8*)((rdi + 0xADDR)) = 1;
                        }
                    }
                } else {
                    sub_140028010();
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
