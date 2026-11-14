// L1 structure recovery (14 blocks)
void sub_14001fa28_l1(void) {
    esi = ecx;
    xmm0 = 0;
    rect = xmm0;
    SetCursor(qword_ADDR);
    rdi = rax;
    if (((esi >> 0xADDR) & 1) != 0) {
        sub_14000cbc8(1);
    }
    var_2c = 0;
    var_30 = 0;
    SendMessageW(qword_ADDR, 0xADDR, &var_2c, &var_30, 0);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    eax += 1;
    r15 = (int64)(eax);
    LocalAlloc(2, (r15 + r15));
    rbx = rax;
    if (rax != 0) {
        GetClientRect(qword_ADDR, &rect);
        LocalLock(rbx);
        r14 = rax;
        SendMessageW(qword_ADDR, 0xADDR, r15, rax);
        sub_1400125a0(rect.right, rect.bottom, esi, qword_ADDR);
        rsi = rax;
        if (rax != 0) {
            r15d = 0xADDR;
            GetWindowLongW(qword_ADDR, 0xADDR);
            SetWindowLongW(rsi, r15d, eax);
            SendMessageW(rsi, 0xADDR, qword_ADDR, 0xADDR);
            SendMessageW(rsi, (r15 + 0xADDR), 0, r14);
            if (rax != 0) {
                LocalUnlock(rbx);
                LocalFree(rbx);
                sub_14001f318(rsi);
                SendMessageW(qword_ADDR, 0xADDR, 0, 0);
                rcx = qword_ADDR;
                ebx = 5;
                qword_ADDR = rax;
                ShowWindow(qword_ADDR, 5);
                SendMessageW(qword_ADDR, 0xADDR, 0xADDR, 0);
                SendMessageW(qword_ADDR, 0xADDR, var_2c, var_30);
                SetFocus(qword_ADDR);
                SetCursor(rdi);
                if (dword_ADDR != 0) {
                    xmm0 = 0;
                    rect2 = xmm0;
                    GetClientRect(qword_ADDR, &rect2);
                    edx = rect2.bottom;
                    ecx = rect2.right;
                    edx -= rect2.top;
                    ecx -= rect2.left;
                    sub_14000e848(rcx, rdx);
                    sub_14000f9f0();
                    ShowWindow(qword_ADDR, 5);
                }
                eax = 1;
            } else {
                SetCursor(rdi);
                DestroyWindow(rsi);
                LocalUnlock(rbx);
                LocalFree(rbx);
                eax = 0;
            }
        } else {
            SetCursor(rdi);
        }
    } else {
        SetCursor(rdi);
    }
    return;
}
