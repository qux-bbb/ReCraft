// L1 structure recovery (27 blocks)
void sub_1400088d4_l1(void) {
    r15d = 0;
    rax = &var_30;
    esi = 0xADDR;
    *(u64*)(r8) = 0;
    r9d = 0xADDR;
    rdx = &var_30;
    rcx -= rax;
    r14 = r8;
    do {
        rax = (r9 + 0xADDR);
        if (rax == 0) break;
        eax = (uint32)(*(u16*)((rcx + rdx)));
        if (ax == 0) break;
        *(u16*)(rdx) = ax;
        rdx += 2;
        r9 -= 1;
    } while (r9 != 0);
    rax = (rdx - 2);
    rax = ((r9 != 0) ? rdx : rax);
    *(u16*)(rax) = 0xADDR;
    sub_1400082b0(&var_30, rsi, "_p0");
    OpenSemaphoreW(0xADDR, 0, &var_30);
    rbx = rax;
    if (rax != 0) {
        var_24 = 0;
        var_20 = r15d;
        sub_140005b38(rbx, &var_24);
        edi = eax;
        if (eax >= 0 /* signed */) {
            sub_1400082b0(&var_30, rsi, "h");
            OpenSemaphoreW(0xADDR, 0, &var_30);
            rdi = rax;
            if (rax != 0) {
                sub_140005b38(rdi, &var_20);
                esi = eax;
                if (eax >= 0 /* signed */) {
                    CloseHandle(rdi);
                    if (eax == 0) {
                        sub_1400091e4(var_268, 0xADDR);
                    }
                    rax = (int64)(var_24);
                    rcx = ((int64)(var_20) << 0xADDR);
                    rcx |= rax;
                    *(u64*)(r14) = rcx;
                    CloseHandle(rbx);
                    if (eax == 0) {
                        sub_1400091e4(var_268, 0xADDR);
                    }
                    eax = 0;
                    r11 = &var_250;
                    return;
                }
                r9d = eax;
                sub_140007cf0(var_268, 0xADDR, "wil");
                CloseHandle(rdi);
                if (eax == 0) {
                    sub_1400091e4(var_268, 0xADDR);
                }
                CloseHandle(rbx);
                if (eax == 0) {
                    sub_1400091e4(var_268, 0xADDR);
                }
                eax = esi;
                r11 = &var_250;
                return;
            }
            sub_140007cd0(var_268, 0xADDR, "wil");
            edi = eax;
            CloseHandle(rbx);
            if (eax == 0) {
                sub_1400091e4(var_268, 0xADDR);
            }
            eax = edi;
            r11 = &var_250;
            return;
        }
        r9d = eax;
        sub_140007cf0(var_268, 0xADDR, "wil");
        CloseHandle(rbx);
        if (eax == 0) {
            sub_1400091e4(var_268, 0xADDR);
        }
    }
    GetLastError();
    if (eax == 2) {
        eax = 0;
        r11 = &var_250;
        return;
    }
    sub_140007cd0(var_268, 0xADDR, "wil");
    r11 = &var_250;
    return;
}
