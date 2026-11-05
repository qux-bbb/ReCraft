// L1 structure recovery (40 blocks)
void sub_14000468c_l1(void) {
    *(u64*)(rdx) = 0;
    r15 = rdx;
    rbx = rcx;
    GetCurrentProcessId();
    esi = 0xADDR;
    var_28 = rbx;
    var_20 = 0xADDR;
    sub_1400083f8(&var_50, (rsi - 0xADDR), "Local\\SM0:%lu:%lu:%hs", eax, 0xADDR, rcx);
    var_30 = 0;
    CreateMutexExW(0, &var_50, 0, 0xADDR);
    sub_140009488(&var_30, rax);
    rbx = var_30;
    if (rbx != 0) {
        edx = -1;
        WaitForSingleObjectEx(rbx, (edx | -1), 0);
        if (eax == 0xADDR) {
            r14d = 0;
        } else {
            if ((eax & 0xADDR) != 0) {
                sub_140004f24(var_298);
            }
            r14 = rbx;
        }
        var_48 = 0;
        r8 = &var_48;
        sub_1400088d4(&var_50, rdx);
        edi = eax;
        if (eax >= 0 /* signed */) {
            rax = var_48;
            rcx = (rax * 4);
            if (rcx == 0) {
                *(u64*)(r15) = 0;
                sub_140006634(8, rsi);
                rdi = rax;
                if (rax != 0) {
                    var_38 = 0;
                    sub_140004acc(&var_38, &var_50, rdi);
                    esi = eax;
                    if (eax >= 0 /* signed */) {
                        *(u64*)((rdi + 0xADDR)) = var_38;
                        edx = 0;
                        rax = var_40;
                        *(u64*)((rdi + 8)) = rbx;
                        ebx = 0;
                        var_38 &= 0;
                        var_40 &= 0;
                        *(u64*)((rdi + 0xADDR)) = rax;
                        *(u32*)(rdi) = 1;
                        sub_1400029dc((rdi + 0xADDR), 0, 0xADDR);
                        *(u64*)((rdi + 0xADDR)) = 0;
                        sub_1400039e4((rdi + 0xADDR));
                        rsi = (rdi + 0xADDR);
                        InitializeCriticalSectionEx(rsi, 0, 0);
                        *(u64*)((rsi + 0xADDR)) &= rbx;
                        *(u64*)((rsi + 0xADDR)) &= rbx;
                        *(u64*)((rsi + 0xADDR)) &= rbx;
                        *(u64*)((rsi + 0xADDR)) &= rbx;
                        *(u64*)(r15) = rdi;
                        sub_140003fa0(&var_38);
                        loc_ADDR:
                        if (r14 == 0) {
                            if (rbx == 0) {
                                eax = 0;
                                return;
                            }
                            CloseHandle(rbx);
                            if (eax == 0) {
                                sub_1400091e4(var_298, 0xADDR);
                            }
                        }
                        ReleaseMutex(r14);
                        if (eax == 0) {
                            sub_1400091e4(var_298, 0xADDR);
                        }
                    }
                    r9d = eax;
                    sub_140007cf0(var_298, 0xADDR, "wil");
                    sub_140003fa0(&var_38);
                    GetProcessHeap();
                    HeapFree(rax, 0, rdi);
                    r9d = esi;
                    sub_140007cf0(var_298, 0xADDR, "wil");
                    if (r14 == 0) {
                        if (rbx == 0) {
                            eax = esi;
                            return;
                        }
                        CloseHandle(rbx);
                        if (eax == 0) {
                            sub_1400091e4(var_298, 0xADDR);
                        }
                    }
                    ReleaseMutex(r14);
                    if (eax == 0) {
                        sub_1400091e4(var_298, 0xADDR);
                    }
                }
                r9d = 0xADDR;
                sub_140007cf0(var_298, 0xADDR, "wil");
                r9d = 0xADDR;
                sub_140007cf0(var_298, 0xADDR, "wil");
                if (r14 == 0) {
                    if (rbx == 0) {
                        eax = 0xADDR;
                        return;
                    }
                    CloseHandle(rbx);
                    if (eax == 0) {
                        sub_1400091e4(var_298, 0xADDR);
                    }
                }
                ReleaseMutex(r14);
                if (eax == 0) {
                    sub_1400091e4(var_298, 0xADDR);
                }
                eax = edi;
                return;
            }
            *(u64*)(r15) = rcx;
            ecx = *(u32*)(rcx);
            rax = *(u64*)(r15);
            ecx += 1;
            rbx = var_30;
            *(u32*)(rax) = ecx;
            goto loc_ADDR;
        }
        r9d = eax;
        sub_140007cf0(var_298, 0xADDR, "wil");
        r9d = edi;
        sub_140007cf0(var_298, 0xADDR, "wil");
        r9d = edi;
        sub_140007cf0(var_298, 0xADDR, "wil");
        if (r14 == 0) {
            if (rbx == 0) {
                eax = edi;
                return;
            }
            CloseHandle(rbx);
            if (eax == 0) {
                sub_1400091e4(var_298, 0xADDR);
            }
            eax = edi;
            return;
        }
        ReleaseMutex(r14);
        if (eax == 0) {
            sub_1400091e4(var_298, 0xADDR);
        }
    }
    sub_1400057d8();
    edi = eax;
    if (rbx == 0) {
        eax = edi;
        return;
    }
    CloseHandle(rbx);
    if (eax == 0) {
        sub_1400091e4(var_298, 0xADDR);
    }
}
