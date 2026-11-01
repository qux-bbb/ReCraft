// L1 structure recovery (22 blocks)
void sub_140006d34_l1(void) {
    r15 = r9;
    ebx = r8d;
    r14d = edx;
    rsi = rcx;
    if (*(u8*)(rcx) != 0) {
        sub_140004cb4(rcx);
        if (al != 0) {
            rdi = *(u64*)((rsi + 0xADDR));
            if (ebx == 0xADDR) {
                sub_140006f64(rdi);
                if (byte_ADDR == 0) {
                    rax = qword_ADDR;
                    if (rax == 0) {
                        rbx = (rsi + 0xADDR);
                        AcquireSRWLockExclusive(rbx);
                        sub_140004e74(rsi);
                        if (rbx != 0) {
                            ReleaseSRWLockExclusive(rbx);
                        }
                    } else {
                        sub_140028010();
                    }
                }
            } else if (ebx < 0xADDR) {
                AcquireSRWLockExclusive(rdi);
                if (ebx > 7) {
                    eax = (rbx - 0xADDR);
                    if (eax <= 0xADDR) {
                        sub_140006f2c((rdi + 8), ebx, r14d);
                        bl = *(u8*)((rdi + 0xADDR));
                    } else {
                        sub_140006ef4((rdi + 0xADDR), ebx, r14d, r15d);
                        bl = al;
                    }
                } else {
                    eax = 0xADDR;
                }
                if (rdi != 0) {
                    ReleaseSRWLockExclusive(rdi);
                }
                if (bl != 0) {
                    if (byte_ADDR == 0) {
                        rax = qword_ADDR;
                        if (rax == 0) {
                            rbx = (rsi + 0xADDR);
                            AcquireSRWLockExclusive(rbx);
                            sub_140004e74(rsi);
                            if (rbx != 0) {
                                ReleaseSRWLockExclusive(rbx);
                            }
                        } else {
                            sub_140028010();
                        }
                    }
                    return;
                }
            }
        }
    }
    return;
}
