// L1 structure recovery (45 blocks)
void sub_14001e5ac_l1(void) {
    ebp = edx;
    rbx = rcx;
    rdx = rcx;
    edi = 1;
    sub_14001f57c("/", rcx);
    r14d = 0;
    if (eax != 0) {
        sub_14001f57c("/", rbx);
        if (eax != 0) {
            eax = 0;
            return;
        }
        rbx += 4;
        while (1) {
            eax = (uint32)(*(u16*)(rbx));
            if (ax == 0xADDR) {
                rbx += 2;
                continue;
            }
            if (ax != 9) break;
        }
        esi = edi;
        if (ax == 0) {
            eax = 0;
            return;
        }
        ShowWindow(qword_ADDR, ebp);
        var_70 = 0;
        sub_14001278c(rbx, &var_70);
        rax += 2;
        if (edi != 0) {
            rbx = var_70;
            var_30 = 0;
            var_28 = 0xADDR;
            var_20 = 3;
            CreateFileW(rbx, 0xADDR, 3, 0, 3, 0xADDR, r14);
            qword_ADDR = rax;
            if (rax != -1) {
                r8d = 0;
                sub_140010f48(&var_70, dword_ADDR, 0);
                rbx = var_70;
                if (al == 0) {
                    sub_140010b30(rbx);
                }
                sub_140021324(esi);
            } else {
                GetLastError();
                if (eax == 2) {
                    r8 = qword_ADDR;
                } else if (eax == 5) {
                    r8 = qword_ADDR;
                } else if (eax != 0xADDR) {
                    r8 = qword_ADDR;
                    r8 = ((eax != 0xADDR) ? qword_ADDR : r8);
                }
                var_20 = 0xADDR;
                sub_14000c3a4(qword_ADDR, qword_ADDR, r8, rbx, 0xADDR);
            }
            if (rbx != 0) {
                CoTaskMemFree(rbx);
            }
            eax = 1;
            return;
        }
        ecx = (uint32)(*(u16*)(rax));
        if (cx == 0) {
            rcx = var_70;
            if (rcx == 0) {
                eax = 0;
                return;
            }
            CoTaskMemFree(var_70);
        }
        while (1) {
            if (cx == 0xADDR) {
                rax += 2;
                ecx = (uint32)(*(u16*)(rax));
                continue;
            }
            if (cx != 9) break;
        }
        if (cx == 0xADDR) {
            rdx = (rax + 2);
            eax = 0;
            while (1) {
                ecx = (uint32)(*(u16*)(rdx));
                if (cx == 0) break;
                if (cx == 0xADDR) break;
                r8d = (rax + 1);
                if (r8d >= 0xADDR) break;
                *(u16*)((off_140035850 + (rax * 2))) = cx;
                rdx += 2;
                eax = r8d;
            }
            rax += rax;
            if (rax >= 0xADDR) {
                sub_140001dc8();
            }
            word_ADDR[rax] = r14w;
        }
    }
    rbx += 6;
    while (1) {
        eax = (uint32)(*(u16*)(rbx));
        if (ax == 0xADDR) {
            rbx += 2;
            continue;
        }
        if (ax != 9) break;
    }
    edi = r14d;
    esi = 2;
    if (ax == 0) {
        eax = 0;
        return;
    }
    ShowWindow(qword_ADDR, ebp);
    var_70 = r14;
    sub_14001278c(rbx, &var_70);
    rax += 2;
    if (edi != 0) {
        rbx = var_70;
        var_30 = r14;
        var_28 = 0xADDR;
        var_20 = 3;
        CreateFileW(rbx, 0xADDR, 3, 0, 3, 0xADDR, r14);
        qword_ADDR = rax;
        if (rax != -1) {
            r8d = 0;
            sub_140010f48(&var_70, dword_ADDR, 0);
            rbx = var_70;
            if (al == 0) {
                sub_140010b30(rbx);
            }
            sub_140021324(esi);
        } else {
            GetLastError();
            if (eax == 2) {
                r8 = qword_ADDR;
            } else if (eax == 5) {
                r8 = qword_ADDR;
            } else if (eax != 0xADDR) {
                r8 = qword_ADDR;
                r8 = ((eax != 0xADDR) ? qword_ADDR : r8);
            }
            var_20 = 0xADDR;
            sub_14000c3a4(qword_ADDR, qword_ADDR, r8, rbx, 0xADDR);
        }
        if (rbx != 0) {
            CoTaskMemFree(rbx);
        }
        eax = 1;
        return;
    }
    eax = 0;
    return;
}
