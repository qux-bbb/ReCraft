// L1 structure recovery (20 blocks)
void sub_1400090c8_l1(void) {
    al = *(u8*)((rcx + 2));
    r14d = 0;
    rbx = *(u64*)(rdx);
    rdi = r8;
    r15 = rdx;
    rsi = rcx;
    if (al != 1) {
        if (al != 2) {
            if (*(u16*)(rsi) != r14w) {
                rbp = (rsi + 8);
                while (1) {
                    r9d = (uint32)(*(u16*)(rbp));
                    rax = (r9 + rbx);
                    if (rax <= rdi) {
                        rdi -= rbx;
                        sub_1400096fc(rbx, rdi, *(u64*)((rsi + 0xADDR)));
                        ecx = (uint32)(*(u16*)(rbp));
                        al = 1;
                        rcx += rbx;
                        *(u64*)(r15) = rcx;
                    } else {
                        al = 0;
                    }
                } else {
                    r14 = (rbx + 2);
                    if (r14 > rdi) {
                        al = 0;
                        return;
                    }
                    rbp = (rsi + 8);
                    rdx = rdi;
                    rdx -= rbx;
                    r9d = 2;
                    sub_1400096fc(rbx, rdx, rbp);
                    rbx = r14;
                }
            }
        } else {
            rbp = (rbx + 4);
            if (rbp > rdi) {
                al = 0;
                return;
            }
            rax = (rcx + 4);
            if (rbx == 0) {
                _o__errno();
                *(u32*)(rax) = 0xADDR;
                sub_140002934();
            } else if (rax == 0) {
                *(u32*)(rbx) = eax;
            } else {
                *(u32*)(rbx) = *(u32*)(rax);
            }
            rbx = rbp;
        }
    } else {
        rbp = (rbx + 2);
        if (rbp > r8) {
            al = 0;
            return;
        }
        eax = (uint32)(*(u16*)((rcx + 4)));
        if (rbx == 0) {
            _o__errno();
            *(u32*)(rax) = 0xADDR;
            sub_140002934();
        }
    }
    return;
}
