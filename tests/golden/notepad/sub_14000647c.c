// L1 structure recovery (17 blocks)
void sub_14000647c_l1(void) {
    rbp = rcx;
    rsi = rdx;
    AcquireSRWLockShared(rdx);
    rdi = *(u64*)((rbp + 0xADDR));
    rdi -= *(u64*)((rbp + 0xADDR));
    rdi >>= 4;
    if (rsi != 0) {
        ReleaseSRWLockShared(rsi);
    }
    ebx = 0;
    if (rdi != 0) {
        do {
            r14d = 0;
            r15d = 0;
            EnterCriticalSection(rbp);
            AcquireSRWLockExclusive(rsi);
            if (rbx < rdi) {
                rdx = *(u64*)((rbp + 0xADDR));
                rax = (rbx << 4);
                rax += rdx;
                do {
                    rcx = (rbx + 1);
                    if (*(u64*)(rax) != r14) {
                        rbx += rbx;
                        xmm1 = *(__m128*)((rdx + (rbx * 8)));
                        rbx = rcx;
                        xmm0 = xmm1;
                        r14 = xmm1;
                        xmm0 >>= 8;
                        r15 = xmm0;
                        break;
                    }
                    rax += 0xADDR;
                    rbx = rcx;
                } while (rcx < rdi);
            }
            if (rsi != 0) {
                ReleaseSRWLockExclusive(rsi);
            }
            if (r14 != 0) {
                rcx = r15;
                rax = r14;
                sub_140028010();
            }
            if (rbp != 0) {
                LeaveCriticalSection(rbp);
            }
        } while (rbx < rdi);
    }
    return;
}
