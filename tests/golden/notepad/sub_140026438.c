// L1 structure recovery (38 blocks)
void sub_140026438_l1(void) {
    var_60 = rdx;
    esi = 0;
    var_30 = rsi;
    sub_1400258d4(rcx, &var_30);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rcx = var_30;
        var_38 = rsi;
        rax = *(u64*)(rcx);
        rax = *(u64*)(rax);
        sub_140028010(var_30, off_14002af98, &var_38);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            var_40 = rsi;
            hstring = rsi;
            WindowsCreateStringReference("Windows.Security.EnterpriseData.FileProtectionManager", 0xADDR, &hstringHeader, &hstring);
            if (eax < 0 /* signed */) {
                ecx = eax;
                sub_14000b320(eax);
            }
            RoGetActivationFactory(hstring, off_14002e2b0, &var_40);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                rbx = var_40;
                var_48 = rsi;
                rax = *(u64*)(rbx);
                rdi = *(u64*)((rax + 0xADDR));
                sub_140009b18(&hstringHeader, &var_60);
                r8 = *(u64*)((rax + 0xADDR));
                rax = rdi;
                sub_140028010(rbx, var_38, *(u64*)((rax + 0xADDR)), &var_48);
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    rdi = var_48;
                    var_50 = rsi;
                    sub_140024ebc(rdi, rdx, r8);
                    ebx = eax;
                    if (eax >= 0 /* signed */) {
                        rax = *(u64*)(rdi);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(rdi, &var_50);
                        ebx = eax;
                    }
                    if (ebx >= 0 /* signed */) {
                        rcx = var_50;
                        var_58 = esi;
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(var_50, &var_58);
                        ebx = eax;
                        if (eax >= 0 /* signed */) {
                            ebx = esi;
                            rcx = var_50;
                            bl = (uint8)((var_58 != 3));
                            if (rcx != 0) {
                                var_50 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_48;
                            if (rcx != 0) {
                                var_48 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_40;
                            if (rcx != 0) {
                                var_40 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_38;
                            if (rcx != 0) {
                                var_38 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_30;
                            if (rcx != 0) {
                                var_30 = rsi;
                                rdx = *(u64*)(rcx);
                                rax = *(u64*)((rdx + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                        } else {
                            edx = 0xADDR;
                            r9d = ebx;
                            sub_140007cf0(var_b8, rdx, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
                            rcx = var_50;
                            if (rcx != 0) {
                                var_50 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_48;
                            if (rcx != 0) {
                                var_48 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_40;
                            if (rcx != 0) {
                                var_40 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_38;
                            if (rcx != 0) {
                                var_38 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_30;
                            if (rcx != 0) {
                                var_30 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                                eax = ebx;
                                return;
                            }
                        }
                    } else {
                        edx = 0xADDR;
                    }
                } else {
                    r9d = eax;
                    sub_140007cf0(var_b8, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
                }
            } else {
                r9d = eax;
                sub_140007cf0(var_b8, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
            }
            eax = ebx;
            return;
        }
        r9d = eax;
        sub_140007cf0(var_b8, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
    }
    r9d = eax;
    sub_140007cf0(var_b8, (rsi + 0xADDR), "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
}
