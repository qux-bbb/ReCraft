// L1 structure recovery (12 blocks)
void sub_14002193c_l1(void) {
    var_98 = edx;
    var_90 = rcx;
    rcx = qword_ADDR;
    ebx = 0;
    ebp = r9d;
    esi = r8d;
    r12d = edx;
    if (rcx != 0) {
        wcsnlen(qword_ADDR, 0xADDR);
        rdi = rax;
    } else {
        edi = ebx;
    }
    eax = edi;
    var_a8 = r12d;
    eax *= ebp;
    eax -= esi;
    eax += r12d;
    eax = ((rax * 2) + 2);
    if (eax > 0 /* signed */) {
        r13 = (int64)(eax);
        GlobalAlloc(0, r13);
        rsi = rax;
        if (rax != 0) {
            var_a0 = ebx;
            r15 = rax;
            if (ebp <= 0 /* signed */) {
                rbp = var_90;
            } else {
                r13d = var_a8;
                r12 = rbp;
                rbp = var_90;
                r14 = (int64)(edi);
                r14 += r14;
                do {
                    rax = &var_a0;
                    var_20 = rax;
                    sub_14001f8d8(rbp, r13d, off_140036540);
                    ecx = var_a0;
                    rbx = rax;
                    rbx -= rbp;
                    rdx = rbp;
                    rbx >>= 1;
                    rdi = rax;
                    ecx += ebx;
                    rbx += rbx;
                    r13d -= ecx;
                    r8 = rbx;
                    rcx = r15;
                    sub_1400274ac(r15, rbp, rbx);
                    r15 += rbx;
                    r8 = r14;
                    sub_1400274ac(r15, qword_ADDR);
                    rax = (int64)(var_a0);
                    r15 += r14;
                    rbp = (rdi + (rax * 2));
                    r12 -= 1;
                } while (r12 != 0);
                r12d = var_98;
            }
            rax = r15;
            rdx = rbp;
            rax -= rsi;
            rcx = r15;
            rax >>= 1;
            rax += rax;
            r13 -= rax;
            r8 = r13;
            sub_1400274ac(rcx, rdx, r8);
            SendMessageW(qword_ADDR, 0xADDR, 0, (int64)(r12d));
            SendMessageW(qword_ADDR, 0xADDR, 1, rsi);
            GlobalFree(rsi);
        }
    }
    eax = 1;
    return;
}
