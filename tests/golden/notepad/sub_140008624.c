// L1 structure recovery (13 blocks)
void sub_140008624_l1(void) {
    *(u64*)(rdx) = 0;
    r15 = r8;
    rsi = rdx;
    rbx = rcx;
    if (*(u8*)(rcx) != 0 && byte_ADDR == 0) {
        rax = qword_ADDR;
        if (rax == 0) {
            sub_140004cb4(rbx);
            if (al != 0) {
                rdi = (rbx + 0xADDR);
                AcquireSRWLockExclusive(rdi);
                r14 = (rbx + 0xADDR);
                if (*(u64*)(r14) == 0) {
                    rbp = *(u64*)((rbx + 0xADDR));
                    if (rbp != 0) {
                        *(u64*)(r14) = 0;
                        AcquireSRWLockExclusive(rbp);
                        sub_140008748((rbp + 0xADDR), r14, sub_140002f50, rbx);
                        ReleaseSRWLockExclusive(rbp);
                    }
                }
                r9d = 0;
                sub_140008748((rbx + 0xADDR), rsi, r15, 0);
                rax = *(u64*)(rsi);
                if (rax != 0) {
                    rax |= 0xADDR;
                    *(u64*)(rsi) = rax;
                }
                if (rdi != 0) {
                    ReleaseSRWLockExclusive(rdi);
                }
            }
        } else {
            sub_140028010();
        }
    }
    return;
}
