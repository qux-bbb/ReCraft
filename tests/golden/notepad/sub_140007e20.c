// L1 structure recovery (54 blocks)
void sub_140007e20_l1(void) {
    *(u32*)((rcx + 4)) = r8d;
    r13d = 0;
    eax = *(u32*)((rdx + 8));
    rsi = rcx;
    *(u32*)((rcx + 8)) = eax;
    r12 = -1;
    *(u64*)((rcx + 0xADDR)) = 0;
    r15 = rdx;
    eax = (uint32)(*(u16*)((rdx + 0xADDR)));
    *(u16*)((rcx + 0xADDR)) = ax;
    *(u8*)((rcx + 0xADDR)) = *(u8*)(rdx);
    *(u64*)((rcx + 0xADDR)) = 0;
    *(u64*)((rcx + 0xADDR)) = *(u64*)((rdx + 0xADDR));
    rax = *(u64*)((rdx + 0xADDR));
    *(u64*)((rcx + 0xADDR)) = rax;
    *(u64*)((rcx + 0xADDR)) = 0;
    rcx = *(u64*)((rdx + 0xADDR));
    if (rcx != 0) {
        rax = r12;
        do {
            rax += 1;
        } while (*(u8*)((rcx + rax)) != r13b);
        rax += 1;
    } else {
        eax = (rcx + 1);
    }
    rdx = *(u64*)((rdx + 0xADDR));
    if (rdx != 0) {
        rcx = r12;
        do {
            rcx += 1;
        } while (*(u8*)((rdx + rcx)) != r13b);
        rcx += 1;
    } else {
        ecx = (rdx + 1);
    }
    r8 = *(u64*)((r15 + 0xADDR));
    if (r8 != 0) {
        rdx = r12;
        do {
            rdx += 1;
        } while (*(u16*)((r8 + (rdx * 2))) != r13w);
        rdx = ((rdx * 2) + 2);
    } else {
        edx = (r8 + 2);
    }
    rbp = (rdx + rcx);
    rbp += rax;
    rdi = (rsi + 0xADDR);
    if (*(u64*)((rsi + 0xADDR)) == r13) {
        sub_140006634(8, rbp);
        r14 = rax;
        if (rax != 0) {
            rbx = *(u64*)((rsi + 0xADDR));
            GetProcessHeap();
            HeapFree(rax, 0, rbx);
            *(u64*)((rsi + 0xADDR)) = r14;
            *(u64*)(rdi) = rbp;
        }
    }
    rbx = *(u64*)((rsi + 0xADDR));
    if (rbx != 0) {
        rdx = *(u64*)(rdi);
        rdi = (rsi + 0xADDR);
        r8 = *(u64*)((r15 + 0xADDR));
        r14 = (rdx + rbx);
        if (rbx == r14) {
            if (rdi != 0) {
                *(u64*)(rdi) = r13;
            }
        } else if (r8 != 0) {
            if (*(u8*)(r8) != r13b) {
                rbp = r12;
                do {
                    rbp += 1;
                } while (*(u8*)((r8 + rbp)) != r13b);
                rbp += 1;
                if (rdx >= rbp) {
                    r9 = rbp;
                    sub_1400096fc(rbx, rdx, r8);
                    if (rdi != 0) {
                        *(u64*)(rdi) = rbx;
                    }
                    rbx += rbp;
                }
            }
        }
        r8 = *(u64*)((r15 + 0xADDR));
        rdi = (rsi + 0xADDR);
        if (rbx == r14) {
            if (rdi != 0) {
                *(u64*)(rdi) = r13;
            }
        } else if (r8 != 0) {
            if (*(u8*)(r8) != r13b) {
                rbp = r12;
                do {
                    rbp += 1;
                } while (*(u8*)((r8 + rbp)) != r13b);
                rdx = r14;
                rbp += 1;
                rdx -= rbx;
                if (rdx >= rbp) {
                    r9 = rbp;
                    sub_1400096fc(rbx, rdx, r8);
                    if (rdi != 0) {
                        *(u64*)(rdi) = rbx;
                    }
                    rbx += rbp;
                }
            }
        }
        r8 = *(u64*)((r15 + 0xADDR));
        rdi = (rsi + 0xADDR);
        if (rbx == r14) {
            if (rdi != 0) {
                *(u64*)(rdi) = r13;
            }
        } else if (r8 != 0) {
            if (*(u16*)(r8) != r13w) {
                do {
                    r12 += 1;
                } while (*(u16*)((r8 + (r12 * 2))) != r13w);
                rdx = r14;
                rsi = ((r12 * 2) + 2);
                rdx -= rbx;
                if (rdx >= rsi) {
                    r9 = rsi;
                    sub_1400096fc(rbx, rdx, r8);
                    if (rdi != 0) {
                        *(u64*)(rdi) = rbx;
                    }
                    rbx += rsi;
                }
            }
        }
        r14 -= rbx;
        edx = 0;
        r8 = r14;
        rcx = rbx;
        sub_1400029dc(rbx, 0, r14);
    }
    return;
}
