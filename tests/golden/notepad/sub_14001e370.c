// L1 structure recovery (27 blocks)
void sub_14001e370_l1(void) {
    rbx = rcx;
    rdx = rcx;
    ebp = 0;
    esi = ebp;
    sub_14001f57c("/.SETUP", rcx);
    if (eax != 0) {
        eax = 0;
    } else {
        rcx = qword_ADDR;
        dword_ADDR = 1;
        GetSystemMenu(qword_ADDR, 0);
        rcx = qword_ADDR;
        qword_ADDR = rax;
        LoadAcceleratorsW(qword_ADDR, "SlipUpAcc");
        rcx = qword_ADDR;
        qword_ADDR = rax;
        SetWindowLongW(qword_ADDR, &var_n10, 0xADDR);
        rcx = (rbx + 0xADDR);
        r14d = (rbp + 2);
        while (1) {
            if (*(u16*)(rcx) == 0xADDR) {
                rcx += r14;
                continue;
            }
            if (*(u16*)(rcx) != 9) break;
        }
        if (*(u16*)(rcx) == bp) {
            eax = 6;
        } else {
            var_68 = rbp;
            sub_14001278c(rcx, &var_68);
            rbx = var_68;
            r9d = 0;
            var_30 = rbp;
            var_28 = 0xADDR;
            var_20 = 3;
            CreateFileW(rbx, 0xADDR, (r9 + 3), 0, 3, 0xADDR, rbp);
            qword_ADDR = rax;
            edi = 6;
            if (rax == -1) {
                GetLastError();
                rdx = qword_ADDR;
                r9 = rbx;
                rcx = qword_ADDR;
                if (eax == r14d) {
                    var_20 = 0xADDR;
                    sub_14000c3a4(rcx, rdx, qword_ADDR, rbx, 0xADDR);
                    esi = eax;
                    if (eax == edi) {
                        r9d = 0;
                        var_30 = rbp;
                        var_28 = 0xADDR;
                        var_20 = 4;
                        CreateFileW(rbx, 0xADDR, (r9 + 3), 0, 4, 0xADDR, rbp);
                        qword_ADDR = rax;
                    }
                } else {
                    var_20 = 0xADDR;
                    if (eax == 5) {
                        r8 = qword_ADDR;
                    } else if (eax == 0xADDR) {
                        r8 = qword_ADDR;
                    } else {
                        r8 = qword_ADDR;
                    }
                    sub_14000c3a4(rcx, rdx, r8, rbx);
                    esi = eax;
                }
            }
            if (qword_ADDR != -1) {
                r8d = 0;
                sub_140010f48(&var_68, dword_ADDR, 0);
                rbx = var_68;
                if (al == 0) {
                    sub_140010b30(rbx);
                }
                if (rbx != 0) {
                    CoTaskMemFree(rbx);
                }
                edi = ((esi == r14d) ? r14d : edi);
                eax = edi;
            } else {
                if (rbx == 0) {
                    eax = 0;
                    return;
                }
                CoTaskMemFree(rbx);
            }
        }
    }
    return;
}
