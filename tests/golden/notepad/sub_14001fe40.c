// L1 structure recovery (32 blocks)
void sub_14001fe40_l1(void) {
    var_70 = rcx;
    ebx = 0;
    r13b = dl;
    if (word_ADDR == 0xADDR) {
        al = 0;
    } else {
        var_88 = ebx;
        var_80 = ebx;
        SendMessageW(qword_ADDR, 0xADDR, &var_88, &var_80);
        SendMessageW(qword_ADDR, (rbx + 0xADDR), 0, 0);
        if (var_88 == ebx && var_80 == eax) {
            var_80 = ebx;
        }
        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
        r15 = rax;
        if (rax == 0) {
            al = 0;
            return;
        }
        LocalLock(rax);
        r14 = rax;
        if (rax == 0) {
            al = 0;
            return;
        }
        edi = var_80;
        r12d = ebx;
        ecx = var_88;
        var_70 = ebx;
        if (ecx >= edi) {
            esi = ebx;
        } else {
            esi = edi;
            esi -= ecx;
        }
        if (r13b == 0) {
            rbx = (rax + (rdi * 2));
            GetWindowTextLengthW(qword_ADDR);
            rcx = &var_70;
            eax -= edi;
            var_10 = rcx;
            sub_14001f8d8(rbx, eax, off_140036540);
            rbx = rax;
            if (byte_ADDR != r12b && rax == 0) {
                rax = &var_70;
                var_10 = rax;
                sub_14001f8d8(r14, (rdi + rsi), off_140036540);
                r12d = 1;
                rbx = rax;
            }
        } else {
            if (ecx != edi) {
                edi -= 1;
            }
            rax = &var_70;
            var_10 = rax;
            sub_14001fdc8(r14, edi);
            rbx = rax;
            if (byte_ADDR != r12b && rax == 0) {
                GetWindowTextLengthW(qword_ADDR);
                edx = eax;
                rcx = (int64)(edi);
                eax = esi;
                eax -= edi;
                edx += eax;
                eax = esi;
                rcx -= rax;
                rax = &var_70;
                var_10 = rax;
                rcx = (r14 + (rcx * 2));
                sub_14001fdc8(rcx, rdx);
                r12d = 1;
                return;
            }
        }
        LocalUnlock(r15);
        if (rbx != 0) {
            r9d = var_70;
            rbx -= r14;
            rbx >>= 1;
            r8d = ebx;
            r9d += ebx;
            var_88 = r8d;
            SendMessageW(qword_ADDR, 0xADDR, ebx, (r9d + ebx));
            if ((byte_ADDR & 0xADDR) == 0) {
                SendMessageW(qword_ADDR, 0xADDR, 0, 0);
            }
            if (r12d == 0) {
                rcx = off_14002b210;
            } else if (r13b == 0) {
                rcx = qword_ADDR;
            } else {
                rcx = qword_ADDR;
            }
            sub_14000fcd0(rcx, rdx);
            al = 1;
        } else {
            if ((byte_ADDR & 0xADDR) != 0) {
                al = 0;
                return;
            }
            SetCursor(qword_ADDR);
            r8 = qword_ADDR;
            rcx = qword_ADDR;
            rbx = rax;
            rcx = ((r8 != 0) ? r8 : rcx);
            var_10 = 0xADDR;
            sub_14000c3a4(rcx, qword_ADDR, qword_ADDR, off_140036540, 0xADDR);
            SetCursor(rbx);
        }
    }
    return;
}
