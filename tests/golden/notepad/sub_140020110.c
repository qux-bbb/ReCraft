// L1 structure recovery (18 blocks)
void sub_140020110_l1(void) {
    rbp = rcx;
    edx -= 0xADDR;
    if (edx == 0) {
        GetSystemMenu();
        qword_ADDR = rax;
        if (dword_ADDR == 0) {
            rbx = qword_ADDR;
            rsi = rbx;
            rcx = rbx;
            eax = (uint32)(*(u16*)(rbx));
            while (ax != 0) {
                if (ax == 0xADDR) {
                    rsi = rcx;
                }
                CharNextW();
                rcx = rax;
                eax = (uint32)(*(u16*)(rax));
            }
            r8 = (rsi + 2);
            r8 = ((rsi == rbx) ? rsi : r8);
        } else {
            r8 = qword_ADDR;
        }
        SetDlgItemTextW(rbp, 0xADDR);
        SetFocus(rbp);
        eax = 1;
    } else {
        edx -= 1;
        if (edx == 0) {
            rcx = qword_ADDR;
            ebx = 1;
            dword_ADDR = ebx;
            EnableWindow(qword_ADDR, ebx);
            DestroyWindow(qword_ADDR);
            edi = 0;
            qword_ADDR = rdi;
            rax = rbx;
        } else if (edx == 5) {
            ebx = 1;
            EnableMenuItem(qword_ADDR, 0xADDR, ebx);
            rax = rbx;
            return;
        }
        eax = 0;
    }
    return;
}
