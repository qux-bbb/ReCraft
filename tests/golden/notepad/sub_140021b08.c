// L1 structure recovery (14 blocks)
void sub_140021b08_l1(void) {
    ebx = 0;
    edi = -1;
    if (byte_ADDR != 0xADDR) {
        byte_ADDR = bl;
    }
    SetCursor(qword_ADDR);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    rbp = rax;
    r13d = 1;
    if (rax == 0) {
        ebx = 0;
        do {
            sub_140021cd0(0);
            ebx = ((eax != 0) ? r13d : ebx);
            sub_14001fe40(0, byte_ADDR);
        } while (al != 0);
    } else {
        LocalLock(rax);
        rsi = rax;
        if (rax != 0) {
            GetWindowTextLengthW(qword_ADDR);
            var_60 &= ebx;
            edi = 0;
            r15d = eax;
            r14d = 0;
            rcx = rsi;
            while (1) {
                rax = rcx;
                rax -= rsi;
                edx = r15d;
                rax >>= 1;
                edx -= eax;
                rax = &var_60;
                var_20 = rax;
                sub_14001f8d8(rcx, rdx, off_140036540);
                rcx = rax;
                if (rax == 0) break;
                rax = (int64)(var_60);
                edi += 1;
                r14d += var_60;
                rcx += (rax * 2);
                continue;
            }
            if (edi > 1 /* signed */) {
                sub_14002193c(rsi, r15d, r14d, edi);
                ebx = eax;
            }
        }
        LocalUnlock(rbp);
    }
    SetCursor(qword_ADDR);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    sub_14000fb98(r13b);
    eax = ebx;
    return;
}
