// L1 structure recovery (17 blocks)
void sub_140004acc_l1(void) {
    rbx = r8;
    rbp = rcx;
    r8 = rdx;
    if ((bl & 3) != 0) {
        sub_1400091d4();
    }
    rax = &var_30;
    rbx >>= 2;
    r8 -= rax;
    rcx = &var_30;
    r14d = 0;
    edx = 0xADDR;
    edi = (r14 + 1);
    do {
        rax = (rdx + 0xADDR);
        if (rax == 0) break;
        eax = (uint32)(*(u16*)((r8 + rcx)));
        if (ax == 0) break;
        *(u16*)(rcx) = ax;
        rcx += 2;
        rdx -= rdi;
    } while (rdx != 0);
    rax = (rcx - 2);
    rdx = r15;
    rax = ((rdx != 0) ? rcx : rax);
    *(u16*)(rax) = 0xADDR;
    sub_1400082b0(&var_30, r15, "_");
    rsi = rbx;
    var_28 = 0xADDR;
    r8d = edi;
    rsi >>= 0xADDR;
    ebx &= 0xADDR;
    var_20 = 0;
    r9 = &var_30;
    edx = ebx;
    __asm("cmova r8d, ebx");
    CreateSemaphoreExW(0);
    rbx = rax;
    if (rax != 0) {
        GetLastError();
        sub_140009488(rbp, rbx);
        sub_1400082b0(&var_30, r15, "h");
        var_28 = 0xADDR;
        var_20 = 0;
        edi = ((esi != 0) ? esi : edi);
        CreateSemaphoreExW(0, esi, edi, &var_30, r14d, 0xADDR);
        rbx = rax;
        if (rax != 0) {
            GetLastError();
            sub_140009488((rbp + 8), rbx);
            ebx = r14d;
        } else {
            sub_1400057d8();
            ebx = eax;
            if (eax >= 0 /* signed */) {
                ebx = r14d;
                eax = ebx;
                return;
            }
            edx = 0xADDR;
            r9d = eax;
            sub_140007cf0(var_278, rdx, "wil");
        }
    } else {
        sub_1400057d8();
        ebx = eax;
        if (eax < 0 /* signed */) {
            edx = 0xADDR;
        }
    }
    eax = ebx;
    return;
}
