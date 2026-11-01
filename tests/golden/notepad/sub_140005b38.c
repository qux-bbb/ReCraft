// L1 structure recovery (26 blocks)
void sub_140005b38_l1(void) {
    rsi = rdx;
    rdi = rcx;
    WaitForSingleObject(rcx, 0);
    if (eax != -1) {
        if (eax == 0) {
            var_40 = 0;
            edx = 1;
            rcx = rdi;
            if (eax != 0) {
                var_48 = 0;
                ReleaseSemaphore(rdi, 1, &var_48);
                if (eax != 0) {
                    if (var_48 == 0) {
                        r8d = 0;
                        ReleaseSemaphore(rdi, (r8 + 1), 0);
                        if (eax != 0) {
                            edx = 0xADDR;
                            r9d = 0xADDR;
                            sub_140007cf0(var_28, rdx, "wil");
                            eax = 0xADDR;
                        } else {
                            GetLastError();
                            if (eax != 0xADDR) {
                                edx = 0xADDR;
                                r9d = 0xADDR;
                                sub_140007cf0(var_28, rdx, "wil");
                                eax = 0xADDR;
                                return;
                            }
                            WaitForSingleObject(rdi, 0);
                            if (eax != -1) {
                                if (eax == 0) {
                                    *(u32*)(rsi) = var_40;
                                    eax = 0;
                                } else {
                                    edx = 0xADDR;
                                    r9d = 0xADDR;
                                    sub_140007cf0(var_28, rdx, "wil");
                                    eax = 0xADDR;
                                    return;
                                }
                            } else {
                                edx = 0xADDR;
                                sub_140007cd0(var_28, rdx, "wil");
                            }
                        }
                    } else {
                        edx = 0xADDR;
                        r9d = 0xADDR;
                        sub_140007cf0(var_28, rdx, "wil");
                        eax = 0xADDR;
                        return;
                    }
                } else {
                    edx = 0xADDR;
                    sub_140007cd0(var_28, rdx, "wil");
                    return;
                }
            } else {
                ReleaseSemaphore(rdi, 1, &var_40);
                if (eax != 0) {
                    var_40 += 1;
                    r8d = 0;
                    ReleaseSemaphore(rdi, (r8 + 1), 0);
                    if (eax != 0) {
                        edx = 0xADDR;
                        r9d = 0xADDR;
                        sub_140007cf0(var_28, rdx, "wil");
                        eax = 0xADDR;
                        return;
                    }
                    GetLastError();
                    if (eax == 0xADDR) {
                        *(u32*)(rsi) = var_40;
                        eax = 0;
                        return;
                    }
                    edx = 0xADDR;
                    r9d = 0xADDR;
                    sub_140007cf0(var_28, rdx, "wil");
                    eax = 0xADDR;
                    return;
                }
                edx = 0xADDR;
                sub_140007cd0(var_28, rdx, "wil");
                return;
            }
        } else if (eax != 0xADDR) {
            edx = 0xADDR;
            r9d = 0xADDR;
            sub_140007cf0(var_28, rdx, "wil");
            eax = 0xADDR;
            return;
        }
    } else {
        edx = 0xADDR;
    }
    return;
}
